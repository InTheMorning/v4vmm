//! Search screen view-model projections.
//!
//! These projections keep Discover/Search result display contracts out of
//! `search.rs`, while remaining GPUI-free. The screen owns event wiring,
//! thumbnails, focus, and selection; this module owns the text and image
//! fields that a result row needs to render.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::api::{self, Artist, EntityDetail, Feed, PaymentRoute, Publisher, Track};
use crate::db;
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

/// Borrow-only projection of a [`Publisher`] inspector panel.
///
/// Owns the title fallback (`"Unknown publisher"`), the feed-count and
/// track-count fallbacks (explicit count → collection length → 0),
/// the detail-grid composition, and the feed-list visibility flag.
/// The screen still owns rendering of the feed-list section itself.
pub(crate) struct PublisherInspectorVm<'a> {
    publisher: &'a Publisher,
}

impl<'a> PublisherInspectorVm<'a> {
    #[must_use]
    pub(crate) fn new(publisher: &'a Publisher) -> Self {
        Self { publisher }
    }

    /// Display title — `publisher_text` if present, else
    /// `"Unknown publisher"`.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        self.publisher
            .publisher_text
            .clone()
            .unwrap_or_else(|| "Unknown publisher".to_string())
    }

    /// Number of feeds — `feed_count` if present, else the length of
    /// the embedded `feeds` list, else `0`. Always non-negative; a
    /// negative `feed_count` is clamped to zero so the display never
    /// shows a leading minus.
    #[must_use]
    pub(crate) fn feed_count(&self) -> i32 {
        Self::resolve_count(self.publisher.feed_count, self.publisher.feeds.as_deref())
    }

    /// Number of tracks — same fallback chain as [`Self::feed_count`].
    #[must_use]
    pub(crate) fn track_count(&self) -> i32 {
        Self::resolve_count(self.publisher.track_count, self.publisher.tracks.as_deref())
    }

    fn resolve_count<T>(explicit: Option<i32>, collection: Option<&[T]>) -> i32 {
        explicit
            .or_else(|| collection.map(|c| i32::try_from(c.len()).unwrap_or(i32::MAX)))
            .unwrap_or(0)
            .max(0)
    }

    /// Detail-grid rows in display order: `Feeds`, `Tracks`.
    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        vec![
            ("Feeds".to_string(), self.feed_count().to_string()),
            ("Tracks".to_string(), self.track_count().to_string()),
        ]
    }

    /// Owned copy of the embedded feed list, or an empty `Vec` when
    /// the publisher carries no `feeds` field.
    #[must_use]
    pub(crate) fn feeds(&self) -> Vec<Feed> {
        self.publisher.feeds.clone().unwrap_or_default()
    }

    /// `true` when the publisher carries at least one embedded feed —
    /// used by the screen to decide whether to render the feed-list
    /// section.
    #[must_use]
    pub(crate) fn has_feed_list(&self) -> bool {
        self.publisher
            .feeds
            .as_ref()
            .is_some_and(|feeds| !feeds.is_empty())
    }
}

/// Borrow-only projection over the per-entity action-row state owned by
/// the search inspector. Owns:
/// * the visibility rule (only `feed` and `track` carry an action row);
/// * the four-way subscription button label (busy × subscribed);
/// * the "Add to playlist" toggle label (`feed` adds the noun "feed");
/// * the message-is-error classification used to pick the status colour.
///
/// The screen still owns click handlers, panel state, and rendering;
/// the VM owns the strings and the boolean classifications.
pub(crate) struct ActionRowVm<'a> {
    entity_type: &'a str,
    subscription_busy: bool,
    local_subscription: Option<bool>,
    subscription_message: Option<&'a str>,
}

impl<'a> ActionRowVm<'a> {
    #[must_use]
    pub(crate) fn new(
        entity_type: &'a str,
        subscription_busy: bool,
        local_subscription: Option<bool>,
        subscription_message: Option<&'a str>,
    ) -> Self {
        Self {
            entity_type,
            subscription_busy,
            local_subscription,
            subscription_message,
        }
    }

    /// `true` when an action row should render for this entity type.
    /// Only `feed` and `track` ever do.
    #[must_use]
    pub(crate) fn is_visible(&self) -> bool {
        matches!(self.entity_type, "feed" | "track")
    }

    /// Subscription button label. Distinguishes the busy and idle
    /// states, and routes idle by `entity_type` for the `Feed`/`Track`
    /// noun.
    #[must_use]
    pub(crate) fn subscription_button_label(&self) -> String {
        let subscribed = self.local_subscription.unwrap_or(false);
        if self.subscription_busy {
            return if subscribed {
                "Removing...".into()
            } else {
                "Downloading...".into()
            };
        }
        let noun = if self.entity_type == "feed" {
            "Feed"
        } else {
            "Track"
        };
        if subscribed {
            format!("Remove {noun}")
        } else {
            format!("Download {noun}")
        }
    }

    /// Toggle label for the "Add to playlist" panel. Feeds get the
    /// `Add feed to playlist ▾` form so the operator knows the whole
    /// album will be added.
    #[must_use]
    pub(crate) fn add_to_playlist_label(&self) -> &'static str {
        if self.entity_type == "feed" {
            "Add feed to playlist ▾"
        } else {
            "Add to playlist ▾"
        }
    }

    /// The current subscription status message, if any.
    #[must_use]
    pub(crate) fn subscription_message(&self) -> Option<&str> {
        self.subscription_message
    }

    /// `true` when the current message reads as an error (case-
    /// insensitive substring match on `"error"`). The screen uses this
    /// to pick the danger colour for the message line.
    #[must_use]
    pub(crate) fn message_is_error(&self) -> bool {
        self.subscription_message.is_some_and(|m| {
            // Case-insensitive substring match without an extra alloc:
            // `to_lowercase` only happens once and only on the message.
            m.to_lowercase().contains("error")
        })
    }
}

/// Derive artist result rows from mixed artist/feed/track results.
///
/// Borrow-only projection of one [`api::Contributor`] entry inside the
/// inspector's contributors panel. Owns the `Unknown` name fallback and
/// the `" (role)"` suffix the screen used to inline.
pub(crate) struct ContributorVm<'a> {
    contributor: &'a api::Contributor,
}

impl<'a> ContributorVm<'a> {
    #[must_use]
    pub(crate) fn new(contributor: &'a api::Contributor) -> Self {
        Self { contributor }
    }

    /// Display name with `"Unknown"` fallback.
    #[must_use]
    pub(crate) fn display_name(&self) -> String {
        self.contributor
            .name
            .clone()
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// `" (role)"` suffix when the contributor has a role, otherwise
    /// empty.
    #[must_use]
    pub(crate) fn role_suffix(&self) -> String {
        self.contributor
            .role
            .as_ref()
            .map_or(String::new(), |r| format!(" ({r})"))
    }

    /// `"<name>{ (role)}"` — what the contributor row renders.
    #[must_use]
    pub(crate) fn full_label(&self) -> String {
        format!("{}{}", self.display_name(), self.role_suffix())
    }

    /// Optional clickable href for the row.
    #[must_use]
    pub(crate) fn href(&self) -> Option<&str> {
        self.contributor.href.as_deref()
    }

    /// Group key (used by the screen to bucket contributors). Empty
    /// string means "ungrouped".
    #[must_use]
    pub(crate) fn group(&self) -> &str {
        self.contributor.group_name.as_deref().unwrap_or("")
    }
}

/// Borrow-only projection of one [`api::PaymentRoute`] entry inside the
/// inspector's value-routes panel. Owns the `"Unnamed recipient"` /
/// `"route"` fallbacks, the fee-vs-split classification, and the
/// `"Fees"` / `"Recipients"` group bucket the screen used to inline.
pub(crate) struct PaymentRouteVm<'a> {
    route: &'a PaymentRoute,
}

impl<'a> PaymentRouteVm<'a> {
    #[must_use]
    pub(crate) fn new(route: &'a PaymentRoute) -> Self {
        Self { route }
    }

    #[must_use]
    pub(crate) fn recipient_name(&self) -> String {
        self.route
            .recipient_name
            .clone()
            .unwrap_or_else(|| "Unnamed recipient".to_string())
    }

    #[must_use]
    pub(crate) fn route_type(&self) -> String {
        self.route
            .route_type
            .clone()
            .unwrap_or_else(|| "route".to_string())
    }

    /// Split percentage; `0.0` when the route does not declare one.
    #[must_use]
    pub(crate) fn split(&self) -> f64 {
        self.route.split.unwrap_or_default()
    }

    #[must_use]
    pub(crate) fn is_fee(&self) -> bool {
        self.route.fee.unwrap_or_default()
    }

    /// `"fee"` when the route is marked as a fee, `"split"` otherwise.
    #[must_use]
    pub(crate) fn kind_label(&self) -> &'static str {
        if self.is_fee() {
            "fee"
        } else {
            "split"
        }
    }

    /// Group bucket key — `"Fees"` for fee routes, `"Recipients"`
    /// otherwise.
    #[must_use]
    pub(crate) fn group(&self) -> &'static str {
        if self.is_fee() {
            "Fees"
        } else {
            "Recipients"
        }
    }
}

/// Source of the currently-pushed inspector frame. Used by the screen
/// to colour the back-button affordance and to decide which list the
/// "Back to results" target maps to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorOrigin {
    Recents,
    Search,
}

/// Stateful screen view-model for the Discover (search) tab.
///
/// Mirror of `view_models::library::LibraryViewModel`. Owns the
/// pure-data UI state for the search screen so `SearchApp` shrinks to
/// event wiring and `Render` glue. Per ADR 0023 this struct must
/// remain GPUI-free; fields requiring `gpui::Image`,
/// `gpui::FocusHandle`, or `Entity<_>` stay in `SearchApp`.
///
/// Subsequent commits will move snapshots (`results`,
/// `recent_feeds`, `playlists`).
#[expect(
    clippy::struct_excessive_bools,
    reason = "screen UI flags belong together; splitting them by lint count would obscure the model"
)]
#[derive(Clone, Debug)]
pub(crate) struct SearchViewModel {
    // Search filter / toggle.
    /// Current segmented filter index (`All`, `Artist`, `Feed`,
    /// `Track`). The screen owns the label/value tables; the VM owns
    /// the index and clamps it to the table length on `set_type_filter`.
    pub(crate) type_filter: usize,
    /// Whether the fuzzy-search toggle is on.
    pub(crate) fuzzy_search: bool,
    // Selection / inspector origin.
    /// Selection key — `"<entity_type>:<entity_id>"`. The screen
    /// resolves the key back to an `InspectorFrame` from its loaded
    /// rows.
    pub(crate) selected_key: Option<String>,
    /// Origin of the active inspector frame, if any.
    pub(crate) inspector_origin: Option<InspectorOrigin>,
    // Search-results pane state.
    pub(crate) loading: bool,
    pub(crate) status: String,
    pub(crate) cursor: Option<String>,
    pub(crate) has_more: bool,
    pub(crate) in_flight_tracks: HashSet<String>,
    // Recents pane state.
    pub(crate) recent_loading: bool,
    pub(crate) recent_status: String,
    pub(crate) recent_loaded_once: bool,
    pub(crate) recent_cursor: Option<String>,
    pub(crate) recent_has_more: bool,
    // Layout / drag state.
    pub(crate) resizing: bool,
    // Loaded snapshots — owned here so the screen can become a thin
    // Render impl. None of these carry GPUI types.
    pub(crate) results: Vec<ResultRow>,
    pub(crate) recent_feeds: Vec<Feed>,
    pub(crate) playlists: Vec<db::Playlist>,
}

/// Number of segmented filter slots — see the `TYPE_LABELS` /
/// `TYPE_VALUES` tables in `search.rs`.
const TYPE_FILTER_LEN: usize = 4;

impl SearchViewModel {
    /// Construct a view-model with `SearchApp::new` defaults: `All`
    /// filter, fuzzy search on, no selection, no inspector frame, no
    /// active operation, both panes idle.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            type_filter: 0,
            fuzzy_search: true,
            selected_key: None,
            inspector_origin: None,
            loading: false,
            status: String::new(),
            cursor: None,
            has_more: false,
            in_flight_tracks: HashSet::new(),
            recent_loading: false,
            recent_status: String::new(),
            recent_loaded_once: false,
            recent_cursor: None,
            recent_has_more: false,
            resizing: false,
            results: Vec::new(),
            recent_feeds: Vec::new(),
            playlists: Vec::new(),
        }
    }

    /// Set the segmented filter index. Out-of-range values are
    /// silently ignored — the segmented control owns the range.
    pub(crate) fn set_type_filter(&mut self, index: usize) {
        if index < TYPE_FILTER_LEN {
            self.type_filter = index;
        }
    }

    /// Flip the fuzzy-search toggle.
    pub(crate) fn toggle_fuzzy_search(&mut self) {
        self.fuzzy_search = !self.fuzzy_search;
    }

    /// Set the `inspector_origin` to `Search` — call when pushing a
    /// frame from the main results list.
    pub(crate) fn mark_inspector_from_search(&mut self) {
        self.inspector_origin = Some(InspectorOrigin::Search);
    }

    /// Set the `inspector_origin` to `Recents` — call when pushing a
    /// frame from the "Recent feeds" panel.
    pub(crate) fn mark_inspector_from_recents(&mut self) {
        self.inspector_origin = Some(InspectorOrigin::Recents);
    }

    /// Drop the inspector-origin marker — call when popping the last
    /// frame off the inspector stack.
    pub(crate) fn clear_inspector_origin(&mut self) {
        self.inspector_origin = None;
    }

    /// Replace the selection key.
    pub(crate) fn select(&mut self, key: impl Into<String>) {
        self.selected_key = Some(key.into());
    }

    /// Clear the selection.
    pub(crate) fn clear_selection(&mut self) {
        self.selected_key = None;
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
    fn publisher_inspector_vm_falls_back_to_unknown_publisher_title() {
        let pub_ = Publisher::default();
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.title(), "Unknown publisher");
    }

    #[test]
    fn publisher_inspector_vm_uses_publisher_text_when_present() {
        let pub_ = Publisher {
            publisher_text: Some("Acme Audio".into()),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.title(), "Acme Audio");
    }

    #[test]
    fn publisher_inspector_vm_prefers_explicit_counts_over_collection_length() {
        let pub_ = Publisher {
            feed_count: Some(7),
            track_count: Some(42),
            feeds: Some(vec![Feed::default()]),
            tracks: Some(vec![Track::default(), Track::default()]),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.feed_count(), 7);
        assert_eq!(vm.track_count(), 42);
    }

    #[test]
    fn publisher_inspector_vm_falls_back_to_collection_length_when_count_absent() {
        let pub_ = Publisher {
            feed_count: None,
            track_count: None,
            feeds: Some(vec![Feed::default(), Feed::default()]),
            tracks: Some(vec![Track::default(), Track::default(), Track::default()]),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.feed_count(), 2);
        assert_eq!(vm.track_count(), 3);
    }

    #[test]
    fn publisher_inspector_vm_falls_back_to_zero_when_neither_present() {
        let pub_ = Publisher::default();
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.feed_count(), 0);
        assert_eq!(vm.track_count(), 0);
    }

    #[test]
    fn publisher_inspector_vm_detail_rows_render_in_feeds_then_tracks_order() {
        let pub_ = Publisher {
            feed_count: Some(3),
            track_count: Some(5),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("Feeds".into(), "3".into()));
        assert_eq!(rows[1], ("Tracks".into(), "5".into()));
    }

    #[test]
    fn publisher_inspector_vm_has_feed_list_only_when_feeds_present() {
        let pub_ = Publisher::default();
        let vm = PublisherInspectorVm::new(&pub_);
        assert!(!vm.has_feed_list());
        let pub_ = Publisher {
            feeds: Some(vec![Feed::default()]),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        assert!(vm.has_feed_list());
    }

    #[test]
    fn action_row_vm_visibility_matches_feed_and_track_only() {
        assert!(ActionRowVm::new("feed", false, None, None).is_visible());
        assert!(ActionRowVm::new("track", false, None, None).is_visible());
        assert!(!ActionRowVm::new("artist", false, None, None).is_visible());
        assert!(!ActionRowVm::new("publisher", false, None, None).is_visible());
        assert!(!ActionRowVm::new("release", false, None, None).is_visible());
    }

    #[test]
    fn action_row_vm_busy_label_distinguishes_remove_vs_download() {
        // local_subscription = Some(true) → "Removing..."
        let vm = ActionRowVm::new("feed", true, Some(true), None);
        assert_eq!(vm.subscription_button_label(), "Removing...");
        // local_subscription = Some(false) → "Downloading..."
        let vm = ActionRowVm::new("feed", true, Some(false), None);
        assert_eq!(vm.subscription_button_label(), "Downloading...");
        // local_subscription = None → "Downloading..." (matches the
        // `unwrap_or(false)` in the legacy renderer).
        let vm = ActionRowVm::new("feed", true, None, None);
        assert_eq!(vm.subscription_button_label(), "Downloading...");
    }

    #[test]
    fn action_row_vm_idle_label_picks_noun_by_entity_type() {
        let vm = ActionRowVm::new("feed", false, Some(false), None);
        assert_eq!(vm.subscription_button_label(), "Download Feed");
        let vm = ActionRowVm::new("track", false, Some(false), None);
        assert_eq!(vm.subscription_button_label(), "Download Track");
        let vm = ActionRowVm::new("feed", false, Some(true), None);
        assert_eq!(vm.subscription_button_label(), "Remove Feed");
        let vm = ActionRowVm::new("track", false, Some(true), None);
        assert_eq!(vm.subscription_button_label(), "Remove Track");
    }

    #[test]
    fn action_row_vm_idle_label_treats_unknown_local_subscription_as_downloadable() {
        let vm = ActionRowVm::new("feed", false, None, None);
        assert_eq!(vm.subscription_button_label(), "Download Feed");
    }

    #[test]
    fn action_row_vm_add_to_playlist_label_uses_feed_noun_for_feeds() {
        let vm = ActionRowVm::new("feed", false, None, None);
        assert_eq!(vm.add_to_playlist_label(), "Add feed to playlist ▾");
        let vm = ActionRowVm::new("track", false, None, None);
        assert_eq!(vm.add_to_playlist_label(), "Add to playlist ▾");
    }

    #[test]
    fn action_row_vm_message_is_error_when_text_contains_error_token() {
        let vm = ActionRowVm::new("feed", false, None, Some("Subscribed!"));
        assert!(!vm.message_is_error());
        let vm = ActionRowVm::new("feed", false, None, Some("error: bad request"));
        assert!(vm.message_is_error());
        let vm = ActionRowVm::new("feed", false, None, Some("Error: bad request"));
        assert!(vm.message_is_error());
        let vm = ActionRowVm::new("feed", false, None, None);
        assert!(!vm.message_is_error());
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

    #[test]
    fn search_view_model_starts_with_all_filter_fuzzy_on_and_no_selection() {
        let vm = SearchViewModel::new();
        assert_eq!(vm.type_filter, 0);
        // Production default — `SearchApp::new` set fuzzy_search = true
        // and the VM mirrors that.
        assert!(vm.fuzzy_search);
        assert_eq!(vm.selected_key, None);
        assert_eq!(vm.inspector_origin, None);
    }

    #[test]
    fn search_view_model_starts_with_idle_panes_and_no_in_flight_tracks() {
        let vm = SearchViewModel::new();
        assert!(!vm.loading);
        assert!(vm.status.is_empty());
        assert_eq!(vm.cursor, None);
        assert!(!vm.has_more);
        assert!(vm.in_flight_tracks.is_empty());
        assert!(!vm.recent_loading);
        assert!(vm.recent_status.is_empty());
        assert!(!vm.recent_loaded_once);
        assert_eq!(vm.recent_cursor, None);
        assert!(!vm.recent_has_more);
        assert!(!vm.resizing);
    }

    #[test]
    fn contributor_vm_falls_back_to_unknown_name() {
        let c = api::Contributor::default();
        let vm = ContributorVm::new(&c);
        assert_eq!(vm.display_name(), "Unknown");
        assert_eq!(vm.role_suffix(), "");
        assert_eq!(vm.full_label(), "Unknown");
        assert_eq!(vm.href(), None);
    }

    #[test]
    fn contributor_vm_combines_name_and_role_suffix() {
        let c = api::Contributor {
            name: Some("Ada".into()),
            role: Some("producer".into()),
            ..api::Contributor::default()
        };
        let vm = ContributorVm::new(&c);
        assert_eq!(vm.display_name(), "Ada");
        assert_eq!(vm.role_suffix(), " (producer)");
        assert_eq!(vm.full_label(), "Ada (producer)");
    }

    #[test]
    fn contributor_vm_omits_role_when_absent() {
        let c = api::Contributor {
            name: Some("Ada".into()),
            ..api::Contributor::default()
        };
        let vm = ContributorVm::new(&c);
        assert_eq!(vm.full_label(), "Ada");
    }

    #[test]
    fn contributor_vm_exposes_href_when_present() {
        let c = api::Contributor {
            name: Some("Ada".into()),
            href: Some("https://x".into()),
            ..api::Contributor::default()
        };
        let vm = ContributorVm::new(&c);
        assert_eq!(vm.href(), Some("https://x"));
    }

    #[test]
    fn payment_route_vm_falls_back_to_unnamed_recipient() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.recipient_name(), "Unnamed recipient");
    }

    #[test]
    fn payment_route_vm_route_type_defaults_to_route() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.route_type(), "route");
        let r = api::PaymentRoute {
            route_type: Some("lightning".into()),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.route_type(), "lightning");
    }

    #[test]
    fn payment_route_vm_classifies_fee_vs_split() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert!(!vm.is_fee());
        assert_eq!(vm.kind_label(), "split");
        assert_eq!(vm.group(), "Recipients");

        let r = api::PaymentRoute {
            fee: Some(true),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert!(vm.is_fee());
        assert_eq!(vm.kind_label(), "fee");
        assert_eq!(vm.group(), "Fees");
    }

    #[test]
    fn payment_route_vm_split_value_defaults_to_zero() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert!((vm.split() - 0.0).abs() < f64::EPSILON);
        let r = api::PaymentRoute {
            split: Some(50.0),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert!((vm.split() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn search_view_model_starts_with_empty_snapshots() {
        let vm = SearchViewModel::new();
        assert!(vm.results.is_empty());
        assert!(vm.recent_feeds.is_empty());
        assert!(vm.playlists.is_empty());
    }

    #[test]
    fn search_view_model_set_type_filter_updates_index_and_clears_when_unknown_type() {
        let mut vm = SearchViewModel::new();
        vm.set_type_filter(2);
        assert_eq!(vm.type_filter, 2);
        // Out-of-range index stays at the prior value (caller is the
        // segmented control which knows its range).
        vm.set_type_filter(99);
        assert_eq!(vm.type_filter, 2);
    }

    #[test]
    fn search_view_model_toggle_fuzzy_search_round_trip() {
        let mut vm = SearchViewModel::new();
        // Starts true (production default). Toggling once turns it off.
        vm.toggle_fuzzy_search();
        assert!(!vm.fuzzy_search);
        vm.toggle_fuzzy_search();
        assert!(vm.fuzzy_search);
    }

    #[test]
    fn search_view_model_inspector_origin_remembers_search_vs_recents() {
        let mut vm = SearchViewModel::new();
        vm.mark_inspector_from_search();
        assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Search));
        vm.mark_inspector_from_recents();
        assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Recents));
        vm.clear_inspector_origin();
        assert_eq!(vm.inspector_origin, None);
    }

    #[test]
    fn search_view_model_select_and_clear_selection() {
        let mut vm = SearchViewModel::new();
        vm.select("track:abc");
        assert_eq!(vm.selected_key.as_deref(), Some("track:abc"));
        vm.clear_selection();
        assert_eq!(vm.selected_key, None);
    }
}
