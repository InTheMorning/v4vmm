//! Search screen view-model projections.
//!
//! These projections keep Discover/Search result display contracts out of
//! `search.rs`, while remaining GPUI-free. The screen owns event wiring,
//! thumbnails, focus, and selection; this module owns the text and image
//! fields that a result row needs to render.

#![warn(clippy::pedantic)]

use crate::api::{Artist, EntityDetail, Feed, Publisher};
use crate::view_models::track::TrackVm;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Recording, Release, Track};

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
}
