//! Search screen view-model projections.
//!
//! These projections keep Discover/Search result display contracts out of
//! `search.rs`, while remaining GPUI-free. The screen owns event wiring,
//! thumbnails, focus, and selection; this module owns the text and image
//! fields that a result row needs to render.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::api::{Artist, EntityDetail, Feed, Publisher, Track};
use crate::view_models::track::TrackVm;

/// Search result row data owned by the Discover screen.
#[derive(Clone, Debug)]
pub(crate) struct ResultRow {
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) detail: Option<EntityDetail>,
}

impl ResultRow {
    #[must_use]
    pub(crate) fn new(
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        detail: Option<EntityDetail>,
    ) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            detail,
        }
    }
}

/// Display-ready text and media fields for one Discover result row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResultRowDisplay {
    pub(crate) line1: String,
    pub(crate) line2: String,
    pub(crate) line3: String,
    pub(crate) image_url: Option<String>,
}

/// Borrow-only projection for one Discover result row.
pub(crate) struct ResultRowVm<'a> {
    entity_id: &'a str,
    detail: Option<&'a EntityDetail>,
}

impl<'a> ResultRowVm<'a> {
    #[must_use]
    pub(crate) fn new(entity_id: &'a str, detail: Option<&'a EntityDetail>) -> Self {
        Self { entity_id, detail }
    }

    /// Project API/domain detail into the three-line list-row display used by
    /// Discover results.
    #[must_use]
    pub(crate) fn display(&self) -> ResultRowDisplay {
        match self.detail {
            Some(EntityDetail::Artist(artist)) => self.artist_display(artist),
            Some(EntityDetail::Feed(feed)) => feed_display(feed),
            Some(EntityDetail::Track(track)) => {
                let vm = TrackVm::new(track);
                let line1 = [Some(vm.title()), vm.duration_display()]
                    .into_iter()
                    .flatten()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(" – ");
                let feed_title = track.feed_title.clone().unwrap_or_default();
                let release_artist = track.release_artist.clone().unwrap_or_default();
                let line3 = if release_artist.is_empty() {
                    feed_title
                } else {
                    format!("{feed_title} by {release_artist}")
                };

                ResultRowDisplay {
                    line1,
                    line2: track
                        .track_artist
                        .clone()
                        .unwrap_or_else(|| "Unknown".into()),
                    line3,
                    image_url: track.image_url.clone(),
                }
            }
            Some(EntityDetail::Publisher(publisher)) => publisher_display(publisher),
            Some(EntityDetail::Release(release)) => ResultRowDisplay {
                line1: self.entity_id.to_string(),
                line2: String::new(),
                line3: String::new(),
                image_url: release.image_url.clone(),
            },
            Some(EntityDetail::Recording(recording)) => ResultRowDisplay {
                line1: self.entity_id.to_string(),
                line2: String::new(),
                line3: String::new(),
                image_url: recording.image_url.clone(),
            },
            None => ResultRowDisplay {
                line1: self.entity_id.to_string(),
                line2: String::new(),
                line3: String::new(),
                image_url: None,
            },
        }
    }

    fn artist_display(&self, artist: &Artist) -> ResultRowDisplay {
        let mut parts = Vec::new();
        if let Some(count) = artist.track_count {
            parts.push(count_label(count, "track"));
        }
        if let Some(count) = artist.feed_count {
            parts.push(count_label(count, "feed"));
        }
        let line3 = artist
            .area
            .clone()
            .or_else(|| artist_active_years(artist))
            .unwrap_or_default();

        ResultRowDisplay {
            line1: artist
                .name
                .clone()
                .or_else(|| artist.artist_id.clone())
                .unwrap_or_else(|| self.entity_id.to_string()),
            line2: parts.join(" · "),
            line3,
            image_url: artist.image_url.clone(),
        }
    }
}

fn feed_display(feed: &Feed) -> ResultRowDisplay {
    let count = feed
        .episode_count
        .map_or_else(String::new, |count| format!("{count} tracks"));
    ResultRowDisplay {
        line1: feed_title(feed),
        line2: feed
            .release_artist
            .clone()
            .unwrap_or_else(|| "Unknown".into()),
        line3: count,
        image_url: feed.image_url.clone(),
    }
}

fn publisher_display(publisher: &Publisher) -> ResultRowDisplay {
    let mut parts = Vec::new();
    if let Some(count) = publisher.feed_count {
        parts.push(format!("{count} feeds"));
    }
    if let Some(count) = publisher.track_count {
        parts.push(format!("{count} tracks"));
    }
    ResultRowDisplay {
        line1: publisher.publisher_text.clone().unwrap_or_default(),
        line2: parts.join(" · "),
        line3: String::new(),
        image_url: None,
    }
}

fn count_label(count: i32, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

fn feed_title(feed: &Feed) -> String {
    feed.title
        .clone()
        .or_else(|| feed.name.clone())
        .or_else(|| feed.feed_guid.clone())
        .unwrap_or_else(|| "Untitled".into())
}

fn artist_active_years(artist: &Artist) -> Option<String> {
    match (artist.begin_year, artist.end_year) {
        (Some(begin), Some(end)) => Some(format!("{begin}-{end}")),
        (Some(begin), None) => Some(format!("{begin}-")),
        (None, Some(end)) => Some(format!("until {end}")),
        (None, None) => None,
    }
}

#[must_use]
pub(crate) fn search_result_type_is_visible(entity_type: &str) -> bool {
    matches!(entity_type, "artist" | "feed" | "track")
}

/// Derive artist result rows from mixed artist/feed/track results.
///
/// This is pure projection over already-fetched rows. Network enrichment
/// remains in the screen-side query adapter until a broader command/query
/// layer exists.
#[must_use]
pub(crate) fn artist_rows_from_result_rows(
    rows: &[ResultRow],
    query: Option<&str>,
) -> Vec<ResultRow> {
    let mut artists = BTreeMap::<String, Artist>::new();

    for row in rows {
        match &row.detail {
            Some(EntityDetail::Artist(artist)) => {
                insert_artist_candidate(&mut artists, artist.clone(), query);
            }
            Some(EntityDetail::Feed(feed)) => {
                if let Some(name) = nonempty_text(feed.release_artist.as_deref()) {
                    insert_artist_candidate(
                        &mut artists,
                        Artist {
                            name: Some(name.to_string()),
                            feed_count: Some(1),
                            image_url: feed.image_url.clone(),
                            ..Artist::default()
                        },
                        query,
                    );
                }
            }
            Some(EntityDetail::Track(track)) => {
                insert_track_artist_candidates(&mut artists, track, query);
            }
            Some(
                EntityDetail::Release(_) | EntityDetail::Recording(_) | EntityDetail::Publisher(_),
            )
            | None => {}
        }
    }

    artists
        .into_values()
        .map(|artist| {
            let entity_id = artist
                .name
                .clone()
                .or_else(|| artist.artist_id.clone())
                .unwrap_or_default();
            ResultRow::new("artist", entity_id, Some(EntityDetail::Artist(artist)))
        })
        .collect()
}

fn insert_track_artist_candidates(
    artists: &mut BTreeMap<String, Artist>,
    track: &Track,
    query: Option<&str>,
) {
    let names: BTreeSet<&str> = [
        track.track_artist.as_deref(),
        track.release_artist.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();
    for name in names {
        insert_artist_candidate(
            artists,
            Artist {
                name: Some(name.to_string()),
                track_count: Some(1),
                image_url: track.image_url.clone(),
                ..Artist::default()
            },
            query,
        );
    }
}

fn insert_artist_candidate(
    artists: &mut BTreeMap<String, Artist>,
    artist: Artist,
    query: Option<&str>,
) {
    let Some(name) = artist.name.clone().or_else(|| artist.artist_id.clone()) else {
        return;
    };
    let name = name.trim();
    if name.is_empty() || !artist_name_matches_query(name, query) {
        return;
    }

    let key = name.to_lowercase();
    if let Some(existing) = artists.get_mut(&key) {
        if existing.name.is_none() {
            existing.name = Some(name.to_string());
        }
        if existing.image_url.is_none() {
            existing.image_url = artist.image_url;
        }
        existing.feed_count = add_optional_counts(existing.feed_count, artist.feed_count);
        existing.track_count = add_optional_counts(existing.track_count, artist.track_count);
        return;
    }

    artists.insert(
        key,
        Artist {
            name: Some(name.to_string()),
            ..artist
        },
    );
}

fn artist_name_matches_query(name: &str, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    let normalized_name = name.to_lowercase();
    query
        .split_whitespace()
        .map(str::to_lowercase)
        .all(|term| normalized_name.contains(&term))
}

fn add_optional_counts(left: Option<i32>, right: Option<i32>) -> Option<i32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn nonempty_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Recording, Release};

    #[test]
    fn artist_display_uses_counts_area_and_image() {
        let detail = EntityDetail::Artist(Artist {
            name: Some("The Artist".into()),
            track_count: Some(1),
            feed_count: Some(2),
            area: Some("Canada".into()),
            begin_year: Some(1999),
            image_url: Some("https://example.test/a.png".into()),
            ..Artist::default()
        });

        assert_eq!(
            ResultRowVm::new("artist-id", Some(&detail)).display(),
            ResultRowDisplay {
                line1: "The Artist".into(),
                line2: "1 track · 2 feeds".into(),
                line3: "Canada".into(),
                image_url: Some("https://example.test/a.png".into()),
            }
        );
    }

    #[test]
    fn artist_display_falls_back_to_active_years_then_entity_id() {
        let detail = EntityDetail::Artist(Artist {
            begin_year: Some(2001),
            end_year: None,
            ..Artist::default()
        });

        let display = ResultRowVm::new("artist-id", Some(&detail)).display();
        assert_eq!(display.line1, "artist-id");
        assert_eq!(display.line3, "2001-");
    }

    #[test]
    fn feed_display_uses_title_fallbacks_and_episode_count() {
        let detail = EntityDetail::Feed(Feed {
            name: Some("Feed Name".into()),
            feed_guid: Some("feed-guid".into()),
            release_artist: Some("Release Artist".into()),
            episode_count: Some(12),
            image_url: Some("https://example.test/f.png".into()),
            ..Feed::default()
        });

        assert_eq!(
            ResultRowVm::new("feed-id", Some(&detail)).display(),
            ResultRowDisplay {
                line1: "Feed Name".into(),
                line2: "Release Artist".into(),
                line3: "12 tracks".into(),
                image_url: Some("https://example.test/f.png".into()),
            }
        );
    }

    #[test]
    fn track_display_uses_track_vm_title_duration_and_artist_fallback() {
        let detail = EntityDetail::Track(Track {
            name: Some("Track Name".into()),
            duration_secs: Some(65),
            feed_title: Some("Feed Title".into()),
            release_artist: Some("Release Artist".into()),
            image_url: Some("https://example.test/t.png".into()),
            ..Track::default()
        });

        assert_eq!(
            ResultRowVm::new("track-id", Some(&detail)).display(),
            ResultRowDisplay {
                line1: "Track Name – 1:05".into(),
                line2: "Unknown".into(),
                line3: "Feed Title by Release Artist".into(),
                image_url: Some("https://example.test/t.png".into()),
            }
        );
    }

    #[test]
    fn publisher_display_keeps_no_image_contract() {
        let detail = EntityDetail::Publisher(Publisher {
            publisher_text: Some("Pub".into()),
            feed_count: Some(2),
            track_count: Some(3),
            ..Publisher::default()
        });

        assert_eq!(
            ResultRowVm::new("publisher-id", Some(&detail)).display(),
            ResultRowDisplay {
                line1: "Pub".into(),
                line2: "2 feeds · 3 tracks".into(),
                line3: String::new(),
                image_url: None,
            }
        );
    }

    #[test]
    fn fallback_rows_preserve_release_and_recording_images() {
        let release = EntityDetail::Release(Release {
            image_url: Some("https://example.test/release.png".into()),
            ..Release::default()
        });
        let recording = EntityDetail::Recording(Recording {
            image_url: Some("https://example.test/recording.png".into()),
            ..Recording::default()
        });

        assert_eq!(
            ResultRowVm::new("release-id", Some(&release))
                .display()
                .image_url
                .as_deref(),
            Some("https://example.test/release.png")
        );
        assert_eq!(
            ResultRowVm::new("recording-id", Some(&recording))
                .display()
                .image_url
                .as_deref(),
            Some("https://example.test/recording.png")
        );
        assert_eq!(ResultRowVm::new("bare-id", None).display().line1, "bare-id");
    }

    #[test]
    fn visible_result_types_match_discover_scope() {
        assert!(search_result_type_is_visible("artist"));
        assert!(search_result_type_is_visible("feed"));
        assert!(search_result_type_is_visible("track"));
        assert!(!search_result_type_is_visible("publisher"));
    }

    #[test]
    fn artist_rows_are_derived_from_feed_and_track_details() {
        let rows = vec![
            ResultRow::new(
                "track",
                "track-1",
                Some(EntityDetail::Track(Track {
                    track_artist: Some("The Doerfels".into()),
                    release_artist: Some("The Doerfels".into()),
                    image_url: Some("https://example.test/track.png".into()),
                    ..Track::default()
                })),
            ),
            ResultRow::new(
                "feed",
                "feed-1",
                Some(EntityDetail::Feed(Feed {
                    release_artist: Some("The Doerfels".into()),
                    image_url: Some("https://example.test/feed.png".into()),
                    ..Feed::default()
                })),
            ),
            ResultRow::new(
                "artist",
                "other",
                Some(EntityDetail::Artist(Artist {
                    name: Some("Other Artist".into()),
                    ..Artist::default()
                })),
            ),
        ];

        let artist_rows = artist_rows_from_result_rows(&rows, Some("doerfels"));

        assert_eq!(artist_rows.len(), 1);
        assert_eq!(artist_rows[0].entity_type, "artist");
        assert_eq!(artist_rows[0].entity_id, "The Doerfels");
        let Some(EntityDetail::Artist(artist)) = &artist_rows[0].detail else {
            panic!("expected artist detail");
        };
        assert_eq!(artist.track_count, Some(1));
        assert_eq!(artist.feed_count, Some(1));
        assert_eq!(
            artist.image_url.as_deref(),
            Some("https://example.test/track.png")
        );
    }

    #[test]
    fn artist_rows_merge_case_insensitive_counts() {
        let rows = vec![
            ResultRow::new(
                "feed",
                "feed-1",
                Some(EntityDetail::Feed(Feed {
                    release_artist: Some("Artist".into()),
                    ..Feed::default()
                })),
            ),
            ResultRow::new(
                "track",
                "track-1",
                Some(EntityDetail::Track(Track {
                    track_artist: Some("artist".into()),
                    ..Track::default()
                })),
            ),
        ];

        let artist_rows = artist_rows_from_result_rows(&rows, None);

        assert_eq!(artist_rows.len(), 1);
        let Some(EntityDetail::Artist(artist)) = &artist_rows[0].detail else {
            panic!("expected artist detail");
        };
        assert_eq!(artist.name.as_deref(), Some("Artist"));
        assert_eq!(artist.feed_count, Some(1));
        assert_eq!(artist.track_count, Some(1));
    }
}
