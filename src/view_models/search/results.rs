//! Search result row display and navigation projections.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::api::{Artist, EntityDetail, Feed, Publisher, Track};
use crate::view_models::track::TrackVm;

use super::common::nonempty_text;
use super::ResultNavigationTarget;

/// Search result row data owned by the Discover screen.
#[derive(Clone, Debug)]
pub(crate) struct ResultRow {
    pub(crate) source: SearchResultSource,
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) feed_guid: Option<String>,
    pub(crate) detail: Option<EntityDetail>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SearchResultSource {
    #[default]
    MusicIndex,
    Library,
}

impl ResultRow {
    #[must_use]
    pub(crate) fn new(
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        detail: Option<EntityDetail>,
    ) -> Self {
        Self {
            source: SearchResultSource::MusicIndex,
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            feed_guid: None,
            detail,
        }
    }

    #[must_use]
    pub(crate) fn musicindex_track(
        track_guid: impl Into<String>,
        feed_guid: Option<String>,
        detail: Option<EntityDetail>,
    ) -> Self {
        Self {
            source: SearchResultSource::MusicIndex,
            entity_type: "track".into(),
            entity_id: track_guid.into(),
            feed_guid,
            detail,
        }
    }

    #[must_use]
    pub(crate) fn local_library_track(track_id: i64, detail: EntityDetail) -> Self {
        Self {
            source: SearchResultSource::Library,
            entity_type: "track".into(),
            entity_id: track_id.to_string(),
            feed_guid: None,
            detail: Some(detail),
        }
    }

    #[must_use]
    pub(crate) fn key(&self) -> String {
        source_entity_key(
            self.source,
            &self.entity_type,
            &self.entity_id,
            self.feed_guid.as_deref(),
        )
    }

    #[must_use]
    pub(crate) fn display(&self) -> ResultRowDisplay {
        let mut display = ResultRowVm::new(&self.entity_id, self.detail.as_ref()).display();
        let source = match self.source {
            SearchResultSource::MusicIndex => "index",
            SearchResultSource::Library => "library",
        };
        let id = scoped_entity_id(
            &self.entity_type,
            &self.entity_id,
            self.feed_guid.as_deref(),
        );
        display.element_id = format!("result-item:{source}:{}:{}", self.entity_type, id);
        display.kind_label.clone_from(&self.entity_type);
        display
    }

    #[must_use]
    pub(crate) fn render_item(&self) -> ResultRowRenderItem {
        ResultRowRenderItem {
            selection_key: self.key(),
            navigation_target: ResultNavigationTarget::from_row(self),
            display: self.display(),
        }
    }

    #[must_use]
    pub(crate) fn inspector_title(&self) -> String {
        let line1 = self.display().line1;
        if line1.is_empty() {
            self.entity_id.clone()
        } else {
            line1
        }
    }
}

pub(super) fn entity_key(entity_type: &str, entity_id: &str) -> String {
    source_entity_key(SearchResultSource::MusicIndex, entity_type, entity_id, None)
}

fn scoped_entity_id(entity_type: &str, entity_id: &str, feed_guid: Option<&str>) -> String {
    match (entity_type, feed_guid) {
        ("track", Some(feed_guid)) if !feed_guid.is_empty() => format!("{feed_guid}:{entity_id}"),
        _ => entity_id.to_string(),
    }
}

pub(super) fn source_entity_key(
    source: SearchResultSource,
    entity_type: &str,
    entity_id: &str,
    feed_guid: Option<&str>,
) -> String {
    let source = match source {
        SearchResultSource::MusicIndex => "index",
        SearchResultSource::Library => "library",
    };
    let entity_id = scoped_entity_id(entity_type, entity_id, feed_guid);
    format!("{source}:{entity_type}:{entity_id}")
}

/// Display-ready text and media fields for one Discover result row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResultRowDisplay {
    pub(crate) element_id: String,
    pub(crate) kind_label: String,
    pub(crate) line1: String,
    pub(crate) line2: String,
    pub(crate) line3: String,
    pub(crate) image_url: Option<String>,
}

/// Complete render projection for one Discover result row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResultRowRenderItem {
    pub(crate) selection_key: String,
    pub(crate) navigation_target: ResultNavigationTarget,
    pub(crate) display: ResultRowDisplay,
}

/// Library-membership state displayed next to search result rows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchLibraryMembership {
    InLibrary,
    NotInLibrary,
}

/// Display contract for search row library-membership status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchLibraryMembershipDisplay {
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) is_in_library: bool,
}

impl SearchLibraryMembership {
    #[must_use]
    pub(crate) const fn display(self) -> SearchLibraryMembershipDisplay {
        match self {
            Self::InLibrary => SearchLibraryMembershipDisplay {
                label: "In Library",
                a11y_label: "Item is in the local library",
                is_in_library: true,
            },
            Self::NotInLibrary => SearchLibraryMembershipDisplay {
                label: "Not in Library",
                a11y_label: "Item is not in the local library",
                is_in_library: false,
            },
        }
    }
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
                    element_id: String::new(),
                    kind_label: String::new(),
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
                element_id: String::new(),
                kind_label: String::new(),
                line1: self.entity_id.to_string(),
                line2: String::new(),
                line3: String::new(),
                image_url: release.image_url.clone(),
            },
            Some(EntityDetail::Recording(recording)) => ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
                line1: self.entity_id.to_string(),
                line2: String::new(),
                line3: String::new(),
                image_url: recording.image_url.clone(),
            },
            None => ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
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
            element_id: String::new(),
            kind_label: String::new(),
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
        element_id: String::new(),
        kind_label: String::new(),
        line1: feed_display_title(feed),
        line2: nonempty_text(feed.release_artist.as_deref())
            .or_else(|| nonempty_text(feed.publisher_text.as_deref()))
            .map_or_else(|| "Unknown".into(), str::to_string),
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
        element_id: String::new(),
        kind_label: String::new(),
        line1: publisher.publisher_text.clone().unwrap_or_default(),
        line2: parts.join(" · "),
        line3: String::new(),
        image_url: None,
    }
}

fn count_label(count: i32, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

#[must_use]
pub(crate) fn feed_display_title(feed: &Feed) -> String {
    nonempty_text(feed.title.as_deref())
        .or_else(|| nonempty_text(feed.name.as_deref()))
        .or_else(|| nonempty_text(feed.feed_guid.as_deref()))
        .map_or_else(|| "Untitled".into(), str::to_string)
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

#[must_use]
pub(crate) fn normalized_search_query(value: &str) -> Option<String> {
    let query = value.trim();
    if query.chars().any(char::is_alphanumeric) {
        Some(query.to_string())
    } else {
        None
    }
}

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
