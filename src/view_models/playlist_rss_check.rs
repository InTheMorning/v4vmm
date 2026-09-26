//! Playlist RSS check action and report display (ADR 0076 Decision 2).
//!
//! This module projects the playlist RSS check actor snapshot into the
//! "Check RSS" action and the check report of the playlist page. The
//! report names the feed, the host and the HTTP status of each result. It
//! gives the recorded start and finish times of the run.
//!
//! ADR 0076 packet 002 adds the applied differences. Each difference names
//! the feed, the track, the field, the old value, the new value and the
//! check time. Each stale feed gets a podping.me link. The app sends no
//! podping.
//!
//! ADR 0076 packet 005 adds a "Copy feed URL" action beside each podping.me
//! link, and it names the requested wait of each host that the check
//! stopped after a `Retry-After` value.

#![warn(clippy::pedantic)]

use serde_json::Value;

use crate::runtime::{
    wait_seconds, DifferenceKind, HostStopReason, PlaylistRssCheckSnapshot, PlaylistRssRun,
    RssCheckRunState, RssCheckTrigger, RssFeedCheck, RssFeedOutcome, RssField, StoppedHost,
    StoredDifference,
};

/// The podping.me page. ADR 0075 Decision I and ADR 0076 Decision 4 direct
/// the operator there. The app sends no podping.
pub(crate) const PODPING_URL: &str = "https://podping.me/";

/// The longest value text that one difference row shows. The stored
/// difference row keeps the full value.
const VALUE_TEXT_LIMIT: usize = 160;

/// Typed availability of the "Check RSS" action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaylistRssCheckAvailability {
    Available,
    /// A check of this playlist runs.
    Running,
    /// The playlist has no track, so it has no feed to check.
    EmptyPlaylist,
    /// The background runtime is not available.
    RuntimeUnavailable,
}

/// Display contract of the "Check RSS" action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistCheckRssActionDisplay {
    pub(crate) button_id: String,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) availability: PlaylistRssCheckAvailability,
    pub(crate) enabled: bool,
}

impl PlaylistCheckRssActionDisplay {
    #[must_use]
    pub(crate) fn new(playlist_id: i64, availability: PlaylistRssCheckAvailability) -> Self {
        let a11y_label = match availability {
            PlaylistRssCheckAvailability::Available => "Check the RSS feeds of this playlist",
            PlaylistRssCheckAvailability::Running => {
                "The RSS check of this playlist runs. Wait for its result"
            }
            PlaylistRssCheckAvailability::EmptyPlaylist => {
                "The playlist has no tracks, so it has no RSS feed to check"
            }
            PlaylistRssCheckAvailability::RuntimeUnavailable => {
                "The RSS check is not available because the background runtime did not start"
            }
        };
        Self {
            button_id: format!("playlist-check-rss-{playlist_id}"),
            label: "Check RSS",
            a11y_label,
            availability,
            enabled: availability == PlaylistRssCheckAvailability::Available,
        }
    }
}

/// Availability of the action from the playlist state and the snapshot.
#[must_use]
pub(crate) fn availability(
    playlist_id: i64,
    track_count: usize,
    snapshot: Option<&PlaylistRssCheckSnapshot>,
) -> PlaylistRssCheckAvailability {
    let Some(snapshot) = snapshot else {
        return PlaylistRssCheckAvailability::RuntimeUnavailable;
    };
    if snapshot.is_running(playlist_id) {
        PlaylistRssCheckAvailability::Running
    } else if track_count == 0 {
        PlaylistRssCheckAvailability::EmptyPlaylist
    } else {
        PlaylistRssCheckAvailability::Available
    }
}

/// Role of one feed row. The row text states the result, so the role is
/// never the only signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaylistRssFeedRowRole {
    Waiting,
    Document,
    NotModified,
    Failed,
    NotChecked,
}

/// One feed row of the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistRssFeedRowDisplay {
    pub(crate) id: String,
    pub(crate) feed: String,
    pub(crate) result: String,
    pub(crate) role: PlaylistRssFeedRowRole,
}

/// Typed availability of the download action of an added track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AddedTrackDownloadAvailability {
    Available,
    /// The track was deleted after the check.
    TrackMissing,
}

/// The existing download action on a `track_added` difference row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AddedTrackDownloadDisplay {
    pub(crate) button_id: String,
    pub(crate) track_id: Option<i64>,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: String,
    pub(crate) availability: AddedTrackDownloadAvailability,
    pub(crate) enabled: bool,
}

/// One applied difference of the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistRssDifferenceRowDisplay {
    pub(crate) id: String,
    /// The feed, and the track when the difference belongs to a track.
    pub(crate) subject: String,
    pub(crate) feed: String,
    pub(crate) track: Option<String>,
    pub(crate) field: &'static str,
    pub(crate) old_value: String,
    pub(crate) new_value: String,
    /// The change in words: the old value, the new value and what the app did.
    pub(crate) change: String,
    /// The recorded check time, in UTC.
    pub(crate) time: String,
    pub(crate) download: Option<AddedTrackDownloadDisplay>,
}

/// Typed availability of the "Copy feed URL" action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CopyFeedUrlAvailability {
    Available,
    /// The stored difference has no feed URL.
    NoFeedUrl,
}

/// The "Copy feed URL" action beside the podping.me link of a stale feed
/// (ADR 0076 packet 005). The renderer puts `feed_url` on the clipboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CopyFeedUrlDisplay {
    pub(crate) button_id: String,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: String,
    pub(crate) availability: CopyFeedUrlAvailability,
    pub(crate) enabled: bool,
    /// The text that the action puts on the clipboard.
    pub(crate) feed_url: Option<String>,
}

/// The podping.me link of one stale feed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PodpingLinkDisplay {
    pub(crate) id: String,
    pub(crate) feed_id: i64,
    pub(crate) text: String,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: String,
    pub(crate) url: &'static str,
    pub(crate) copy_feed_url: CopyFeedUrlDisplay,
}

/// The report of the latest check of a playlist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistRssCheckReportDisplay {
    pub(crate) id: String,
    pub(crate) heading: &'static str,
    pub(crate) trigger: RssCheckTrigger,
    pub(crate) running: bool,
    pub(crate) summary: String,
    pub(crate) counts: Option<String>,
    pub(crate) rows: Vec<PlaylistRssFeedRowDisplay>,
    pub(crate) stopped_hosts: Vec<String>,
    pub(crate) error: Option<String>,
    /// One sentence that counts the applied differences, when the run has one.
    pub(crate) differences_summary: Option<String>,
    pub(crate) differences: Vec<PlaylistRssDifferenceRowDisplay>,
    pub(crate) podping_links: Vec<PodpingLinkDisplay>,
}

/// Build the report of the latest run, or `None` when the playlist has no run.
#[must_use]
pub(crate) fn report(
    playlist_id: i64,
    snapshot: Option<&PlaylistRssCheckSnapshot>,
) -> Option<PlaylistRssCheckReportDisplay> {
    let run = snapshot?.run(playlist_id)?;
    Some(PlaylistRssCheckReportDisplay {
        id: format!("playlist-rss-check-report-{playlist_id}"),
        heading: "RSS check",
        trigger: run.trigger,
        running: run.is_running(),
        summary: summary(run),
        counts: (!run.feeds.is_empty()).then(|| counts(run)),
        rows: run
            .feeds
            .iter()
            .map(|feed| row(playlist_id, feed))
            .collect(),
        stopped_hosts: run.stopped_hosts.iter().map(stopped_host).collect(),
        error: run.error.clone(),
        differences_summary: differences_summary(run),
        differences: run
            .differences
            .iter()
            .map(|difference| difference_row(playlist_id, difference))
            .collect(),
        podping_links: podping_links(playlist_id, run),
    })
}

/// Typed action of a playlist row of a removed track (ADR 0076 Decision 7,
/// operator decision 2026-09-24).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlaylistRemovedTrackActionKind {
    /// Remove this one entry, at this position, from this playlist.
    RemoveFromPlaylist {
        /// The playlist of the row.
        playlist_id: i64,
        /// The position of the row in the playlist.
        position: i64,
    },
    /// Ask for confirmation, and then remove the track from each playlist
    /// that holds it.
    RemoveFromAllPlaylists {
        /// Local track database id.
        track_id: i64,
    },
}

/// Typed availability of a playlist row action of a removed track.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlaylistRemovedTrackActionAvailability {
    /// The action can be run.
    Available,
}

impl PlaylistRemovedTrackActionAvailability {
    /// Returns whether a renderer should disable the control.
    #[must_use]
    pub(crate) const fn disabled(self) -> bool {
        match self {
            Self::Available => false,
        }
    }
}

/// One action of a playlist row of a removed track.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistRemovedTrackActionDisplay {
    pub(crate) kind: PlaylistRemovedTrackActionKind,
    pub(crate) id: String,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: String,
    pub(crate) availability: PlaylistRemovedTrackActionAvailability,
}

/// The row error and the two actions of a playlist row of a track with an
/// unconfirmed "removed from feed" mark (ADR 0076 Decision 7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistRemovedFromFeedDisplay {
    /// The row error, with the recorded UTC time of the check.
    pub(crate) error: String,
    pub(crate) remove_from_playlist: PlaylistRemovedTrackActionDisplay,
    pub(crate) remove_from_all_playlists: PlaylistRemovedTrackActionDisplay,
}

/// The row error and actions of a playlist row, when the track has a
/// "removed from feed" mark in the snapshot.
#[must_use]
pub(crate) fn removed_from_feed_row(
    snapshot: Option<&PlaylistRssCheckSnapshot>,
    playlist_id: i64,
    track_id: i64,
    position: i64,
    title: &str,
) -> Option<PlaylistRemovedFromFeedDisplay> {
    let at = snapshot?.removed_mark(playlist_id, track_id)?;
    Some(PlaylistRemovedFromFeedDisplay {
        error: format!(
            "Removed from feed on {}. Remove it from the playlist, or confirm it in the readiness list.",
            time(at)
        ),
        remove_from_playlist: PlaylistRemovedTrackActionDisplay {
            kind: PlaylistRemovedTrackActionKind::RemoveFromPlaylist {
                playlist_id,
                position,
            },
            id: format!("playlist-removed-remove-{playlist_id}-{position}"),
            label: "Remove from playlist",
            a11y_label: format!("Remove {title} from this playlist"),
            availability: PlaylistRemovedTrackActionAvailability::Available,
        },
        remove_from_all_playlists: PlaylistRemovedTrackActionDisplay {
            kind: PlaylistRemovedTrackActionKind::RemoveFromAllPlaylists { track_id },
            id: format!("playlist-removed-remove-all-{playlist_id}-{position}"),
            label: "Remove from all playlists",
            a11y_label: format!("Remove {title} from all playlists"),
            availability: PlaylistRemovedTrackActionAvailability::Available,
        },
    })
}

/// The confirmation of "Remove from all playlists". It names each playlist
/// that holds the track.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RemoveFromAllPlaylistsConfirmationDisplay {
    pub(crate) title: String,
    pub(crate) message: String,
    pub(crate) playlists: Vec<String>,
    pub(crate) cancel_button_id: &'static str,
    pub(crate) cancel_label: &'static str,
    pub(crate) cancel_a11y_label: &'static str,
    pub(crate) confirm_button_id: &'static str,
    pub(crate) confirm_label: &'static str,
    pub(crate) confirm_a11y_label: String,
}

/// Builds the confirmation of "Remove from all playlists" from the stored
/// playlists of the track.
#[must_use]
pub(crate) fn remove_from_all_playlists_confirmation(
    title: &str,
    playlists: &[(i64, String)],
) -> RemoveFromAllPlaylistsConfirmationDisplay {
    let count = playlists.len();
    let noun = if count == 1 { "playlist" } else { "playlists" };
    RemoveFromAllPlaylistsConfirmationDisplay {
        title: format!("Remove {title} from all playlists?"),
        message: format!(
            "The app removes {title} from {count} {noun}. The track stays in the library."
        ),
        playlists: playlists.iter().map(|(_, name)| name.clone()).collect(),
        cancel_button_id: "remove-from-all-playlists-cancel",
        cancel_label: "Cancel",
        cancel_a11y_label: "Keep the track in its playlists",
        confirm_button_id: "remove-from-all-playlists-confirm",
        confirm_label: "Remove",
        confirm_a11y_label: format!("Remove {title} from {count} {noun}"),
    }
}

/// One sentence for a host that the check stopped. It names the host and
/// the requested wait when the response gave one.
fn stopped_host(stopped: &StoppedHost) -> String {
    let host = &stopped.host;
    match stopped.reason {
        HostStopReason::TooManyRequests {
            requested_wait: None,
        } => format!("The check stopped requests to {host} after HTTP 429 (Too Many Requests)."),
        HostStopReason::TooManyRequests {
            requested_wait: Some(wait),
        } => format!(
            "The check stopped requests to {host} after HTTP 429 (Too Many Requests). {host} requested a wait of {} seconds.",
            wait_seconds(wait)
        ),
        HostStopReason::RetryAfterLimit { requested_wait } => format!(
            "The check stopped requests to {host}. {host} requested a wait of {} seconds (Retry-After), and the limit is {} seconds.",
            wait_seconds(requested_wait),
            crate::runtime::playlist_rss_check::MAX_RETRY_AFTER.as_secs()
        ),
    }
}

fn differences_summary(run: &PlaylistRssRun) -> Option<String> {
    if run.differences.is_empty() {
        return None;
    }
    let count = run.differences.len();
    let feeds = stale_feeds(run).len();
    Some(format!(
        "The check wrote {count} RSS {} to the stored values. MusicIndex has older data for {feeds} {}. The app keeps each earlier value as evidence and writes no audio tag.",
        if count == 1 { "value" } else { "values" },
        if feeds == 1 { "feed" } else { "feeds" },
    ))
}

fn stale_feeds(run: &PlaylistRssRun) -> Vec<&StoredDifference> {
    let mut feeds: Vec<&StoredDifference> = Vec::new();
    for difference in &run.differences {
        if !feeds.iter().any(|feed| feed.feed_id == difference.feed_id) {
            feeds.push(difference);
        }
    }
    feeds
}

fn feed_name(difference: &StoredDifference) -> String {
    difference
        .feed_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .or_else(|| difference.feed_url.clone())
        .unwrap_or_else(|| format!("Feed {}", difference.feed_id))
}

fn podping_links(playlist_id: i64, run: &PlaylistRssRun) -> Vec<PodpingLinkDisplay> {
    stale_feeds(run)
        .into_iter()
        .map(|difference| {
            let feed = feed_name(difference);
            let address = difference
                .feed_url
                .clone()
                .unwrap_or_else(|| "the feed address".to_owned());
            let feed_url = difference
                .feed_url
                .as_deref()
                .map(str::trim)
                .filter(|url| !url.is_empty())
                .map(str::to_owned);
            let availability = if feed_url.is_some() {
                CopyFeedUrlAvailability::Available
            } else {
                CopyFeedUrlAvailability::NoFeedUrl
            };
            let copy_feed_url = CopyFeedUrlDisplay {
                button_id: format!(
                    "playlist-rss-check-{playlist_id}-copy-feed-url-{}",
                    difference.feed_id
                ),
                label: "Copy feed URL",
                a11y_label: match availability {
                    CopyFeedUrlAvailability::Available => {
                        format!("Copy the feed URL of {feed} to the clipboard")
                    }
                    CopyFeedUrlAvailability::NoFeedUrl => {
                        format!("The app has no feed URL for {feed}, so it cannot copy it")
                    }
                },
                availability,
                enabled: availability == CopyFeedUrlAvailability::Available,
                feed_url,
            };
            PodpingLinkDisplay {
                id: format!(
                    "playlist-rss-check-{playlist_id}-podping-{}",
                    difference.feed_id
                ),
                feed_id: difference.feed_id,
                text: format!(
                    "MusicIndex has older data for {feed}. Open podping.me and submit {address} to ask MusicIndex to read the feed again. The app sends no podping."
                ),
                label: "Open podping.me",
                a11y_label: format!("Open podping.me in the browser for {feed}"),
                url: PODPING_URL,
                copy_feed_url,
            }
        })
        .collect()
}

fn field_label(field: RssField) -> &'static str {
    match field {
        RssField::Title => "Title",
        RssField::Description => "Description",
        RssField::Artwork => "Artwork",
        RssField::Link => "Link",
        RssField::Enclosure => "Audio file",
        RssField::Duration => "Duration",
        RssField::Date => "Date",
        RssField::Explicit => "Explicit",
        RssField::Language => "Language",
        RssField::AlbumArtist => "Album artist",
        RssField::Artist => "Artist",
        RssField::Owner => "Feed owner",
        RssField::Persons => "Persons",
        RssField::Nostr => "Nostr identity",
        RssField::PaymentRoutes => "Payment routes",
        RssField::Publisher => "Publisher",
        RssField::Track => "Track",
    }
}

fn shorten(text: String) -> String {
    if text.chars().count() <= VALUE_TEXT_LIMIT {
        return text;
    }
    let mut short = text.chars().take(VALUE_TEXT_LIMIT).collect::<String>();
    short.push('…');
    short
}

/// The operator text of one stored value.
fn value_text(field: RssField, value: Option<&Value>) -> String {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return "no value".to_owned();
    };
    let text = match (field, value) {
        (_, Value::String(text)) => text.trim().to_owned(),
        (_, Value::Bool(flag)) => if *flag { "Yes" } else { "No" }.to_owned(),
        (RssField::Enclosure, value) => match (value["url"].as_str(), value["type"].as_str()) {
            (Some(url), Some(mime)) => format!("{url} ({mime})"),
            (Some(url), None) => url.to_owned(),
            (None, Some(mime)) => mime.to_owned(),
            (None, None) => "no value".to_owned(),
        },
        (RssField::Duration, value) => value["seconds"].as_i64().map_or_else(
            || value["raw"].as_str().unwrap_or("no value").to_owned(),
            |seconds| format!("{}:{:02}", seconds / 60, seconds % 60),
        ),
        (RssField::Date, value) => value["text"].as_str().map_or_else(
            || {
                value["instant"]
                    .as_i64()
                    .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
                    .map_or_else(
                        || "no value".to_owned(),
                        |at| at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    )
            },
            str::to_owned,
        ),
        (RssField::Persons, Value::Array(persons)) => persons
            .iter()
            .map(|person| {
                let name = person["name"].as_str().unwrap_or("A person without a name");
                match person["role"].as_str() {
                    Some(role) => format!("{name} ({role})"),
                    None => name.to_owned(),
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
        (RssField::Nostr, Value::Array(ids)) => ids
            .iter()
            .filter_map(|id| id["value"].as_str())
            .collect::<Vec<_>>()
            .join(", "),
        (RssField::PaymentRoutes, value) => {
            let recipients = value["children"]["valueRecipient"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let names = recipients
                .iter()
                .map(|recipient| {
                    let attrs = &recipient["attrs"];
                    format!(
                        "{} {}",
                        attrs["name"]
                            .as_str()
                            .unwrap_or("A recipient without a name"),
                        attrs["split"].as_str().unwrap_or("?")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{} {}: {names}",
                recipients.len(),
                if recipients.len() == 1 {
                    "recipient"
                } else {
                    "recipients"
                }
            )
        }
        (RssField::Publisher, value) => match value["feed_url"].as_str() {
            Some(url) => format!(
                "Publisher feed {} at {url}",
                value["feed_guid"].as_str().unwrap_or("with no GUID")
            ),
            None => format!(
                "Publisher feed {}",
                value["feed_guid"].as_str().unwrap_or("with no GUID")
            ),
        },
        (_, value) => value.to_string(),
    };
    shorten(text)
}

fn difference_row(
    playlist_id: i64,
    difference: &StoredDifference,
) -> PlaylistRssDifferenceRowDisplay {
    let feed = feed_name(difference);
    let track = difference.track_id.map(|track_id| {
        difference
            .track_title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map_or_else(|| format!("Track {track_id}"), str::to_owned)
    });
    let subject = match &track {
        Some(track) => format!("{feed}: {track}"),
        None => feed.clone(),
    };
    let old_value = value_text(difference.field, difference.old_value.as_ref());
    let new_value = value_text(difference.field, difference.new_value.as_ref());
    let (field, change) = match difference.kind {
        DifferenceKind::Changed => (
            field_label(difference.field),
            format!("Old value: {old_value}. New value from RSS: {new_value}. The app wrote the new value."),
        ),
        DifferenceKind::Cleared => (
            field_label(difference.field),
            format!("Old value: {old_value}. RSS no longer states this value, so the app cleared it."),
        ),
        DifferenceKind::TrackAdded => (
            "Track added",
            "RSS lists a new track. The app stored it without a file. Use Download to get the file.".to_owned(),
        ),
        DifferenceKind::TrackRemoved => (
            "Track removed",
            "RSS no longer lists this track. The app marked it \"removed from feed\". The track, its file and its playlist entries stay.".to_owned(),
        ),
        DifferenceKind::TrackReturned => (
            "Track returned",
            "RSS lists this track again. The app cleared its \"removed from feed\" mark.".to_owned(),
        ),
    };
    let download = (difference.kind == DifferenceKind::TrackAdded).then(|| {
        let availability = if difference.track_id.is_some() {
            AddedTrackDownloadAvailability::Available
        } else {
            AddedTrackDownloadAvailability::TrackMissing
        };
        AddedTrackDownloadDisplay {
            button_id: format!(
                "playlist-rss-check-{playlist_id}-download-{}",
                difference.id
            ),
            track_id: difference.track_id,
            label: "Download",
            a11y_label: match availability {
                AddedTrackDownloadAvailability::Available => {
                    format!("Download the added track {subject}")
                }
                AddedTrackDownloadAvailability::TrackMissing => {
                    "The added track is no longer stored, so the app cannot download it".to_owned()
                }
            },
            availability,
            enabled: availability == AddedTrackDownloadAvailability::Available,
        }
    });
    PlaylistRssDifferenceRowDisplay {
        id: format!(
            "playlist-rss-check-{playlist_id}-difference-{}",
            difference.id
        ),
        subject,
        feed,
        track,
        field,
        old_value,
        new_value,
        change,
        time: format!("Checked at {}.", time(difference.recorded_at_us)),
        download,
    }
}

fn time(us: i64) -> String {
    chrono::DateTime::from_timestamp_micros(us).map_or_else(
        || "an unknown time".to_owned(),
        |at| at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    )
}

fn summary(run: &PlaylistRssRun) -> String {
    let started = match run.trigger {
        RssCheckTrigger::Button => format!(
            "The Check RSS button started this check at {}.",
            time(run.started_at_us)
        ),
        RssCheckTrigger::PlaybackStart => format!(
            "Playback from this playlist started this check at {}.",
            time(run.started_at_us)
        ),
    };
    let total = run.feeds.len();
    let state = match run.state {
        RssCheckRunState::Running if total == 0 => {
            "The app reads the feeds of the playlist.".to_owned()
        }
        RssCheckRunState::Running => format!(
            "{} of {total} {} have a result. The check continues.",
            run.completed(),
            feeds(total)
        ),
        RssCheckRunState::Finished => match run.finished_at_us {
            Some(finished) => format!(
                "The check finished at {} with a result for each of {total} {}.",
                time(finished),
                feeds(total)
            ),
            None => format!(
                "The check finished with a result for each of {total} {}.",
                feeds(total)
            ),
        },
        RssCheckRunState::Interrupted => format!(
            "The app stopped before the check had a result for each feed. {} of {total} {} have a result.",
            run.completed(),
            feeds(total)
        ),
        RssCheckRunState::Failed => "The check did not start.".to_owned(),
    };
    format!("{started} {state}")
}

fn feeds(count: usize) -> &'static str {
    if count == 1 {
        "feed"
    } else {
        "feeds"
    }
}

fn counts(run: &PlaylistRssRun) -> String {
    format!(
        "Documents received: {}. Not modified: {}. Failed: {}. Not checked: {}.",
        run.count(RssFeedOutcome::Document),
        run.count(RssFeedOutcome::NotModified),
        run.count(RssFeedOutcome::Failed),
        run.count(RssFeedOutcome::NotChecked),
    )
}

/// Add the stored message of a received response to its row text. The
/// message names a `Retry-After` stop or a comparison failure.
fn with_message(result: String, message: Option<&str>) -> String {
    match message.map(str::trim).filter(|message| !message.is_empty()) {
        Some(message) => format!("{result} {message}"),
        None => result,
    }
}

fn row(playlist_id: i64, feed: &RssFeedCheck) -> PlaylistRssFeedRowDisplay {
    let name = feed
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .or_else(|| feed.feed_url.clone())
        .unwrap_or_else(|| format!("Feed {}", feed.feed_id));
    let host = feed.host.as_deref().unwrap_or("the feed host");
    let status = feed
        .http_status
        .map_or_else(String::new, |status| format!(" HTTP {status}."));
    let (role, result) = match feed.outcome {
        None => (
            PlaylistRssFeedRowRole::Waiting,
            format!("Waiting for the request to {host}."),
        ),
        Some(RssFeedOutcome::Document) => (
            PlaylistRssFeedRowRole::Document,
            with_message(
                format!("{host} sent the RSS document. The app recorded it.{status}"),
                feed.message.as_deref(),
            ),
        ),
        Some(RssFeedOutcome::NotModified) => (
            PlaylistRssFeedRowRole::NotModified,
            with_message(
                format!("{host} reported no change since the last check.{status}"),
                feed.message.as_deref(),
            ),
        ),
        Some(RssFeedOutcome::Failed) => (
            PlaylistRssFeedRowRole::Failed,
            format!(
                "Failed. {}",
                feed.message
                    .clone()
                    .unwrap_or_else(|| format!("The request to {host} failed.{status}"))
            ),
        ),
        Some(RssFeedOutcome::NotChecked) => (
            PlaylistRssFeedRowRole::NotChecked,
            feed.message
                .clone()
                .unwrap_or_else(|| "Not checked.".to_owned()),
        ),
    };
    PlaylistRssFeedRowDisplay {
        id: format!("playlist-rss-check-{playlist_id}-feed-{}", feed.feed_id),
        feed: name,
        result,
        role,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R2-16: the report exposes each difference with the feed, field, old
    /// value, new value, time and podping.me link.
    #[test]
    fn adr_0076_rss_comparison_report_exposes_differences_and_podping_link() {
        use crate::runtime::playlist_rss_check::test_support::snapshot_with_differences;
        let difference = |id: i64,
                          track_id: Option<i64>,
                          field,
                          kind,
                          old: Option<Value>,
                          new: Option<Value>| {
            StoredDifference {
                id,
                run_id: 1,
                feed_id: 7,
                feed_title: Some("Album".into()),
                feed_url: Some("https://feed.test/album.xml".into()),
                track_id,
                track_title: track_id.map(|_| "Song".to_owned()),
                field,
                kind,
                old_value: old,
                new_value: new,
                recorded_at_us: 1_790_000_000_000_000,
            }
        };
        let snapshot = snapshot_with_differences(
            3,
            vec![
                difference(
                    1,
                    None,
                    RssField::Title,
                    DifferenceKind::Changed,
                    Some(Value::from("Old album")),
                    Some(Value::from("New album")),
                ),
                difference(
                    2,
                    Some(9),
                    RssField::Track,
                    DifferenceKind::TrackAdded,
                    None,
                    Some(Value::from("Song")),
                ),
            ],
            &[(11, 1_790_000_000_000_000)],
        );
        let report = report(3, Some(&snapshot)).unwrap();
        assert_eq!(report.differences.len(), 2);
        let title = &report.differences[0];
        assert_eq!(title.feed, "Album");
        assert_eq!(title.track, None);
        assert_eq!(title.field, "Title");
        assert_eq!(title.old_value, "Old album");
        assert_eq!(title.new_value, "New album");
        assert!(title.change.contains("Old value: Old album."));
        assert!(title.change.contains("New value from RSS: New album."));
        assert_eq!(title.time, "Checked at 2026-09-21 14:13:20 UTC.");
        assert!(title.download.is_none());
        let added = &report.differences[1];
        assert_eq!(added.subject, "Album: Song");
        let download = added.download.as_ref().unwrap();
        assert!(download.enabled);
        assert_eq!(download.track_id, Some(9));
        assert_eq!(report.podping_links.len(), 1);
        let link = &report.podping_links[0];
        assert_eq!(link.url, "https://podping.me/");
        assert_eq!(link.feed_id, 7);
        assert!(link.text.contains("https://feed.test/album.xml"));
        assert!(link.text.contains("The app sends no podping."));
        assert!(report
            .differences_summary
            .as_deref()
            .is_some_and(|summary| summary.contains("2 RSS values")));
        assert!(removed_from_feed_row(Some(&snapshot), 3, 11, 0, "Song")
            .is_some_and(|row| row.error.starts_with("Removed from feed on 2026-09-21 ")));
        assert_eq!(
            removed_from_feed_row(Some(&snapshot), 3, 12, 1, "Song"),
            None
        );
    }

    #[test]
    fn adr_0076_playlist_check_availability_follows_run_and_tracks() {
        let snapshot = PlaylistRssCheckSnapshot::default();
        assert_eq!(
            availability(1, 3, Some(&snapshot)),
            PlaylistRssCheckAvailability::Available
        );
        assert_eq!(
            availability(1, 0, Some(&snapshot)),
            PlaylistRssCheckAvailability::EmptyPlaylist
        );
        assert_eq!(
            availability(1, 3, None),
            PlaylistRssCheckAvailability::RuntimeUnavailable
        );
        let action = PlaylistCheckRssActionDisplay::new(1, PlaylistRssCheckAvailability::Running);
        assert!(!action.enabled);
        assert_eq!(action.label, "Check RSS");
    }

    /// R3-13: a playlist row of a removed track exposes the row error and
    /// the two typed playlist actions with accessibility labels.
    #[test]
    fn adr_0076_route_readiness_playlist_row_exposes_error_and_two_actions() {
        use crate::runtime::playlist_rss_check::test_support::snapshot_with_differences;

        // 2026-09-24 10:00:00 UTC, the recorded time of a check.
        let snapshot = snapshot_with_differences(3, Vec::new(), &[(11, 1_790_244_000_000_000)]);
        let row = removed_from_feed_row(Some(&snapshot), 3, 11, 4, "Gone Song").unwrap();
        assert_eq!(
            row.error,
            "Removed from feed on 2026-09-24 10:00:00 UTC. Remove it from the playlist, or confirm it in the readiness list."
        );
        assert_eq!(
            row.remove_from_playlist.kind,
            PlaylistRemovedTrackActionKind::RemoveFromPlaylist {
                playlist_id: 3,
                position: 4
            }
        );
        assert_eq!(row.remove_from_playlist.label, "Remove from playlist");
        assert_eq!(
            row.remove_from_playlist.a11y_label,
            "Remove Gone Song from this playlist"
        );
        assert!(!row.remove_from_playlist.availability.disabled());
        assert_eq!(
            row.remove_from_all_playlists.kind,
            PlaylistRemovedTrackActionKind::RemoveFromAllPlaylists { track_id: 11 }
        );
        assert_eq!(
            row.remove_from_all_playlists.label,
            "Remove from all playlists"
        );
        assert_eq!(
            row.remove_from_all_playlists.a11y_label,
            "Remove Gone Song from all playlists"
        );
        assert_ne!(
            row.remove_from_playlist.id,
            row.remove_from_all_playlists.id
        );
        assert_eq!(
            removed_from_feed_row(Some(&snapshot), 3, 12, 5, "Other"),
            None
        );
        assert_eq!(removed_from_feed_row(None, 3, 11, 4, "Gone Song"), None);
    }

    /// R3-14: the confirmation names each playlist that holds the track.
    #[test]
    fn adr_0076_route_readiness_remove_everywhere_confirmation_names_each_playlist() {
        let display = remove_from_all_playlists_confirmation(
            "Gone Song",
            &[(1, "Friday Show".to_owned()), (4, "Warm Up".to_owned())],
        );
        assert_eq!(display.playlists, vec!["Friday Show", "Warm Up"]);
        assert_eq!(
            display.message,
            "The app removes Gone Song from 2 playlists. The track stays in the library."
        );
        assert_eq!(
            display.confirm_a11y_label,
            "Remove Gone Song from 2 playlists"
        );
    }

    /// R5-03: the report exposes "Copy feed URL" for each stale feed, with
    /// typed availability and an accessibility label. The action carries
    /// the feed URL.
    #[test]
    fn adr_0076_follow_up_report_exposes_copy_feed_url_for_each_stale_feed() {
        use crate::runtime::playlist_rss_check::test_support::snapshot_with_differences;
        let difference = |id: i64, feed_id: i64, feed_url: Option<&str>| StoredDifference {
            id,
            run_id: 1,
            feed_id,
            feed_title: Some(format!("Album {feed_id}")),
            feed_url: feed_url.map(str::to_owned),
            track_id: None,
            track_title: None,
            field: RssField::Title,
            kind: DifferenceKind::Changed,
            old_value: Some(Value::from("Old")),
            new_value: Some(Value::from("New")),
            recorded_at_us: 1_790_000_000_000_000,
        };
        let snapshot = snapshot_with_differences(
            3,
            vec![
                difference(1, 7, Some("https://feed.test/seven.xml")),
                difference(2, 7, Some("https://feed.test/seven.xml")),
                difference(3, 8, Some("https://feed.test/eight.xml")),
                difference(4, 9, None),
            ],
            &[],
        );
        let display = report(3, Some(&snapshot)).unwrap();
        assert_eq!(display.podping_links.len(), 3);
        let seven = &display.podping_links[0].copy_feed_url;
        assert_eq!(seven.label, "Copy feed URL");
        assert_eq!(seven.availability, CopyFeedUrlAvailability::Available);
        assert!(seven.enabled);
        assert_eq!(
            seven.feed_url.as_deref(),
            Some("https://feed.test/seven.xml")
        );
        assert_eq!(
            seven.a11y_label,
            "Copy the feed URL of Album 7 to the clipboard"
        );
        let eight = &display.podping_links[1].copy_feed_url;
        assert_eq!(
            eight.feed_url.as_deref(),
            Some("https://feed.test/eight.xml")
        );
        assert_ne!(seven.button_id, eight.button_id);
        let missing = &display.podping_links[2].copy_feed_url;
        assert_eq!(missing.availability, CopyFeedUrlAvailability::NoFeedUrl);
        assert!(!missing.enabled);
        assert_eq!(missing.feed_url, None);
        assert!(!missing.a11y_label.is_empty());
        assert_eq!(display.podping_links[0].url, PODPING_URL);
    }

    /// R5-02, report side: the report names the stopped host and the
    /// requested wait.
    #[test]
    fn adr_0076_follow_up_report_names_host_and_requested_wait() {
        use crate::runtime::playlist_rss_check::test_support::snapshot_with_stopped_host;
        use crate::runtime::StoppedHost;
        use std::time::Duration;
        let snapshot = snapshot_with_stopped_host(
            3,
            StoppedHost {
                host: "limited.test".to_owned(),
                reason: HostStopReason::RetryAfterLimit {
                    requested_wait: Duration::from_secs(61),
                },
            },
        );
        let limited = report(3, Some(&snapshot)).unwrap();
        assert_eq!(
            limited.stopped_hosts,
            ["The check stopped requests to limited.test. limited.test requested a wait of 61 seconds (Retry-After), and the limit is 60 seconds."]
        );
        let snapshot = snapshot_with_stopped_host(
            3,
            StoppedHost {
                host: "busy.test".to_owned(),
                reason: HostStopReason::TooManyRequests {
                    requested_wait: Some(Duration::from_millis(300_500)),
                },
            },
        );
        let busy = report(3, Some(&snapshot)).unwrap();
        assert_eq!(
            busy.stopped_hosts,
            ["The check stopped requests to busy.test after HTTP 429 (Too Many Requests). busy.test requested a wait of 301 seconds."]
        );
    }
}
