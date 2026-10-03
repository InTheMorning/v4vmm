//! Subscription materialization and retained conversion input (ADR 0066).

pub(crate) mod materialization;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::api::{track_with_feed_defaults, Client, Feed, SourceEntityLink, Track};
use crate::audio_tags::{read_audio_tags, write_id3v24_edits, Id3v24Edit};
use crate::config;
use crate::db::{self, TrackRow};
use crate::identity_ingest;
use crate::library_path::LibraryRelativePath;
use crate::metadata::{
    sanitize_feed_source_text, sanitize_track_context_source_text, sanitize_track_source_text,
    source_text_missing, TagCompareResult, TrackContext,
};
use crate::rss;
use crate::track_compare::{download_track, local_track_path, select_audio_enclosure};

const MUSICINDEX_TRACK_PERSISTENCE_INCLUDE: &str =
    "source_enclosures,source_links,source_ids,source_contributors,payment_routes";

#[derive(Clone)]
pub enum SubscribeTrackRequest {
    LibraryTrack {
        track: Box<TrackRow>,
    },
    SearchTrack {
        track_context: Box<TrackContext>,
        musicindex_endpoint: crate::config::MusicIndexEndpoint,
        mark_feed_subscribed: bool,
        return_tag_compare: bool,
    },
}

pub struct SubscribeTrackOutcome {
    pub path: PathBuf,
    pub relative_path: Option<LibraryRelativePath>,
    pub format_warning: Option<String>,
    pub conversion: crate::audio_format::ConversionOutcome,
    pub applied_edits: usize,
    pub marked_downloaded: bool,
    pub compare: Option<TagCompareResult>,
}

pub struct SubscribeFeedRequest {
    pub feed: Feed,
    pub musicindex_endpoint: crate::config::MusicIndexEndpoint,
}

pub struct SubscribeFeedOutcome {
    pub downloaded: usize,
    pub applied_edits: usize,
    pub skipped: usize,
}

struct SearchTrackSubscription {
    track_context: TrackContext,
    persistence_track: Option<Track>,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    mark_feed_subscribed: bool,
    return_tag_compare: bool,
}

pub(crate) enum PreparedTrack {
    Existing { path: PathBuf },
    Downloaded(Box<crate::track_compare::DownloadedTrack>),
}

impl PreparedTrack {
    fn working_path(&self) -> &Path {
        match self {
            PreparedTrack::Existing { path } => path.as_path(),
            PreparedTrack::Downloaded(d) => d.path.as_path(),
        }
    }

    fn format_warning(&self) -> Option<String> {
        match self {
            PreparedTrack::Existing { .. } => None,
            PreparedTrack::Downloaded(d) => d.format_warning.clone(),
        }
    }
}

pub(crate) fn subscribe_track_retaining(
    conn: Arc<Mutex<Connection>>,
    cfg: &config::DownloadConfig,
    request: SubscribeTrackRequest,
    retained: &mut Option<materialization::Materialization>,
) -> Result<SubscribeTrackOutcome> {
    match request {
        SubscribeTrackRequest::LibraryTrack { track } => {
            subscribe_library_track_internal(conn, cfg, *track, retained)
        }
        SubscribeTrackRequest::SearchTrack {
            track_context,
            musicindex_endpoint,
            mark_feed_subscribed,
            return_tag_compare,
        } => subscribe_track_from_search_internal(
            conn,
            cfg,
            SearchTrackSubscription {
                track_context: *track_context,
                persistence_track: None,
                musicindex_endpoint,
                mark_feed_subscribed,
                return_tag_compare,
            },
            retained,
        ),
    }
}

pub(crate) fn subscribe_feed_retaining(
    conn: Arc<Mutex<Connection>>,
    cfg: &config::DownloadConfig,
    request: SubscribeFeedRequest,
    mut retain: impl FnMut(
        SubscribeTrackRequest,
        Option<materialization::Materialization>,
        &Result<SubscribeTrackOutcome>,
    ) -> Result<()>,
) -> Result<SubscribeFeedOutcome> {
    let mut feed = request.feed;
    sanitize_feed_source_text(&mut feed);
    let musicindex_endpoint = request.musicindex_endpoint;
    let feed_url = feed
        .feed_url
        .clone()
        .filter(|url| !source_text_missing(Some(url.as_str())))
        .ok_or_else(|| anyhow!("feed has no RSS URL"))?;

    {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        rss::subscribe_feed(&mut db, &feed_url, &musicindex_endpoint)?;
        identity_ingest::persist_musicindex_context_by_feed_url(
            &mut db,
            &feed_url,
            Some(&feed),
            None,
        )?;
    }

    let api_client = Client::new_with_base_url(musicindex_endpoint.clone());
    let mut downloaded = 0usize;
    let mut applied_edits = 0usize;
    let mut skipped = 0usize;
    let local_tracks = local_feed_tracks(&conn, &feed_url)?;
    let tracks = feed.tracks.clone().unwrap_or_default();
    let track_count = tracks.len();

    for (index, track) in tracks.into_iter().enumerate() {
        let mut original_track = track;
        let local_track =
            local_track_for_feed_download(&original_track, &local_tracks, index, track_count);
        fill_missing_download_source(&mut original_track, local_track);
        let mut track_for_metadata = original_track.clone();
        let mut track_for_persistence = original_track.clone();
        if let Some(track_guid) = track_for_metadata.track_guid.as_deref() {
            if let Ok(hydrated) = api_client.fetch_track(
                track_guid,
                Some(
                    "source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes",
                ),
            ) {
                track_for_persistence = hydrated.clone();
                track_for_metadata = hydrated;
                fill_missing_download_source(&mut track_for_persistence, local_track);
                fill_missing_download_source(&mut track_for_metadata, local_track);
            }
        }
        sanitize_track_source_text(&mut track_for_persistence);
        sanitize_track_source_text(&mut track_for_metadata);
        {
            let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
            identity_ingest::persist_musicindex_context_by_feed_url(
                &mut db,
                &feed_url,
                None,
                Some(&track_for_persistence),
            )?;
        }
        let track = track_with_feed_defaults(track_for_metadata, Some(&feed));
        // ADR 0076 Decision 10: this context only finds and fetches the
        // enclosure. The materialization reads RSS and the stored values.
        let mut track_context = TrackContext::new(track, Some(feed.clone()));
        sanitize_track_context_source_text(&mut track_context);
        let error_title = track_context
            .track
            .title
            .clone()
            .unwrap_or_else(|| "(untitled)".to_owned());

        let original_request = SubscribeTrackRequest::SearchTrack {
            track_context: Box::new(track_context.clone()),
            musicindex_endpoint: musicindex_endpoint.clone(),
            mark_feed_subscribed: true,
            return_tag_compare: false,
        };
        let mut retained = None;
        let result = subscribe_track_from_search_internal(
            Arc::clone(&conn),
            cfg,
            SearchTrackSubscription {
                track_context,
                persistence_track: Some(track_for_persistence),
                musicindex_endpoint: musicindex_endpoint.clone(),
                mark_feed_subscribed: true,
                return_tag_compare: false,
            },
            &mut retained,
        );
        retain(original_request, retained, &result)?;
        match result {
            Ok(outcome) => {
                if outcome.marked_downloaded {
                    downloaded += 1;
                } else {
                    skipped += 1;
                }
                applied_edits += outcome.applied_edits;
            }
            Err(err) => {
                eprintln!("skip {}: {err:#}", error_title);
                skipped += 1;
            }
        }
    }

    if track_count > 0 && downloaded == 0 {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        db::set_feed_subscribed_by_url(&db, &feed_url, false)?;
        return Err(anyhow!(
            "Downloaded feed had {track_count} tracks but none could be downloaded/tagged; reverted download"
        ));
    }

    Ok(SubscribeFeedOutcome {
        downloaded,
        applied_edits,
        skipped,
    })
}

fn local_feed_tracks(conn: &Arc<Mutex<Connection>>, feed_url: &str) -> Result<Vec<TrackRow>> {
    let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    let Some(feed_id) = db::feed_id_by_url(&db, feed_url)? else {
        return Ok(Vec::new());
    };
    db::feed_tracks(&db, feed_id)
}

fn local_track_for_feed_download<'a>(
    track: &Track,
    local_tracks: &'a [TrackRow],
    index: usize,
    remote_track_count: usize,
) -> Option<&'a TrackRow> {
    track
        .track_guid
        .as_deref()
        .and_then(|track_guid| {
            local_tracks
                .iter()
                .find(|local| local.item_guid == track_guid)
        })
        .or_else(|| {
            track.enclosure_url.as_deref().and_then(|enclosure_url| {
                local_tracks
                    .iter()
                    .find(|local| local.enclosure_url.as_deref() == Some(enclosure_url))
            })
        })
        .or_else(|| {
            track.track_number.and_then(|track_number| {
                local_tracks.iter().find(|local| {
                    local.track_number == Some(i64::from(track_number))
                        && track
                            .title
                            .as_deref()
                            .is_none_or(|title| local.track_title.as_deref() == Some(title))
                })
            })
        })
        .or_else(|| {
            (local_tracks.len() == remote_track_count)
                .then(|| local_tracks.get(index))
                .flatten()
        })
}

fn fill_missing_download_source(track: &mut Track, local_track: Option<&TrackRow>) {
    if let Some(local_track) = local_track {
        fill_missing_download_source_from_local_row(track, local_track);
    }
}

fn fill_missing_download_source_from_local_row(track: &mut Track, local: &TrackRow) {
    if source_text_missing(track.enclosure_url.as_deref()) {
        track.enclosure_url.clone_from(&local.enclosure_url);
    }
    if source_text_missing(track.enclosure_type.as_deref()) {
        track.enclosure_type.clone_from(&local.enclosure_type);
    }
    if source_text_missing(track.image_url.as_deref()) {
        track.image_url.clone_from(&local.track_image_href);
    }
    if track.track_number.is_none() {
        track.track_number = local.track_number.and_then(|track_number| {
            i32::try_from(track_number)
                .ok()
                .filter(|track_number| *track_number > 0)
        });
    }
    if source_text_missing(track.title.as_deref()) {
        track.title.clone_from(&local.track_title);
    }
    if source_text_missing(track.feed_title.as_deref()) {
        track.feed_title.clone_from(&local.feed_title);
    }
}

fn subscribe_library_track_internal(
    conn: Arc<Mutex<Connection>>,
    cfg: &config::DownloadConfig,
    track: TrackRow,
    retained: &mut Option<materialization::Materialization>,
) -> Result<SubscribeTrackOutcome> {
    let track_context = TrackContext::new(track_row_to_api_track(&track), None);
    *retained = Some(materialization::Materialization::new(
        track,
        track_context,
        cfg.music_dir.clone(),
    ));
    retained
        .as_mut()
        .expect("created materialization")
        .run(&conn, cfg, false, false)
}

fn subscribe_track_from_search_internal(
    conn: Arc<Mutex<Connection>>,
    cfg: &config::DownloadConfig,
    input: SearchTrackSubscription,
    retained: &mut Option<materialization::Materialization>,
) -> Result<SubscribeTrackOutcome> {
    let SearchTrackSubscription {
        track_context,
        persistence_track,
        musicindex_endpoint,
        mark_feed_subscribed,
        return_tag_compare,
    } = input;
    let feed = track_context.feed;
    let original_track = track_context.track.clone();
    let track = track_with_feed_defaults(original_track.clone(), feed.as_ref());
    let mut refreshed_context = TrackContext::new(track, feed);
    refreshed_context.rss_observation = track_context.rss_observation;
    enrich_track_context_from_rss(&mut refreshed_context);
    sanitize_track_context_source_text(&mut refreshed_context);
    let feed_url = refreshed_context
        .feed_url()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("track has no RSS feed URL"))?;
    let track = refreshed_context.track.clone();
    let feed = refreshed_context.feed.clone();
    let api_client = Client::new_with_base_url(musicindex_endpoint.clone());
    let track_for_persistence =
        authoritative_track_for_persistence(persistence_track, &original_track, |track_guid| {
            api_client.fetch_track(track_guid, Some(MUSICINDEX_TRACK_PERSISTENCE_INCLUDE))
        });

    let prior_subscribed = {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        db::feed_is_subscribed_by_url(&db, &feed_url).unwrap_or(false)
    };

    {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        rss::subscribe_feed(&mut db, &feed_url, &musicindex_endpoint)?;
        identity_ingest::persist_musicindex_context_by_feed_url(
            &mut db,
            &feed_url,
            feed.as_ref(),
            track_for_persistence.as_ref(),
        )?;
        if !mark_feed_subscribed && !prior_subscribed {
            db::set_feed_subscribed_by_url(&db, &feed_url, false)?;
        }
    }

    let row = {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        let id = db::find_track_id(
            &db,
            Some(&feed_url),
            track.track_guid.as_deref(),
            track.enclosure_url.as_deref(),
        )?
        .ok_or_else(|| {
            anyhow!("RSS did not identify the requested track; no file was materialized")
        })?;
        db::track_row_by_id(&db, id)?.ok_or_else(|| anyhow!("original track no longer exists"))?
    };
    let mut operation =
        materialization::Materialization::new(row, refreshed_context, cfg.music_dir.clone());
    operation.return_tag_compare = return_tag_compare;
    operation.reconcile_feed = (!mark_feed_subscribed).then(|| feed_url.clone());
    *retained = Some(operation);
    retained
        .as_mut()
        .expect("created materialization")
        .run(&conn, cfg, false, false)
        .inspect_err(|_| {
            if mark_feed_subscribed && !prior_subscribed {
                if let Ok(db) = conn.lock() {
                    let _ = db::set_feed_subscribed_by_url(&db, &feed_url, false);
                }
            }
        })
}

/// A download keeps the track when its tag write fails (ADR 0080 task 004).
/// The function returns the number of applied edits, and the warning for the
/// download result when the write fails.
fn apply_id3_edits_nonfatal(path: &Path, edits: &[Id3v24Edit]) -> (usize, Option<String>) {
    if edits.is_empty() {
        return (0, None);
    }
    match write_id3v24_edits(path, edits) {
        Ok(_) => (edits.len(), None),
        Err(err) => (
            0,
            Some(crate::diagnostics::redact_endpoint_details(&format!(
                "App saved the track but could not write its tags: {err:#}"
            ))),
        ),
    }
}

pub fn track_row_to_api_track(row: &TrackRow) -> Track {
    Track {
        // Identity columns pass through verbatim; the merge boundary owns
        // their placeholder semantics.
        track_guid: Some(row.item_guid.clone()),
        feed_guid: row.feed_guid.clone(),
        // Display facts are sanitized so a polluted DB row cannot become a
        // display string at the metadata grid boundary.
        title: drop_placeholder(row.track_title.clone()),
        track_artist: drop_placeholder(row.artist_name.clone()),
        release_artist: drop_placeholder(row.album_artist_name.clone()),
        feed_title: drop_placeholder(row.feed_title.clone()),
        track_number: row.track_number.and_then(|n| n.try_into().ok()),
        duration_secs: row.duration_seconds.and_then(|s| s.try_into().ok()),
        pub_date: row.pub_date,
        explicit: row.explicit,
        enclosure_url: drop_placeholder(row.enclosure_url.clone()),
        enclosure_type: drop_placeholder(row.enclosure_type.clone()),
        image_url: drop_placeholder(row.track_image_href.clone()),
        publisher_text: None,
        description: None,
        source_links: row
            .transcript_url
            .as_ref()
            .filter(|url| !source_text_missing(Some(url.as_str())))
            .map(|url| {
                vec![SourceEntityLink {
                    entity_type: Some("track".into()),
                    entity_id: Some(row.item_guid.clone()),
                    link_type: Some("transcript".into()),
                    url: Some(url.clone()),
                    ..Default::default()
                }]
            }),
        ..Track::default()
    }
}

/// Strip placeholder transport values (`...`, `\u{2026}`, whitespace-only)
/// so they cannot become display facts in shared metadata renderers.
///
/// Mirrors [`crate::metadata::source_text_missing`] semantics: anything the
/// detector flags as missing is collapsed to `None` even if a previous
/// boundary persisted it into the local DB.
fn drop_placeholder(value: Option<String>) -> Option<String> {
    value.filter(|value| !source_text_missing(Some(value.as_str())))
}

pub fn enrich_track_context_from_rss(context: &mut TrackContext) {
    let _ = enrich_track_context_from_rss_with_recorder(context, None);
}

pub(crate) fn rss_request_spec(
    context: &TrackContext,
) -> Option<crate::provider_observation::ProviderRequestSpec> {
    context.feed_url().map(|url| {
        crate::provider_observation::contracts::rss_request(
            url,
            context.track.track_guid.as_deref(),
            context.track.enclosure_url.as_deref(),
        )
    })
}

pub(crate) fn enrich_track_context_from_rss_with_recorder(
    context: &mut TrackContext,
    recorder: Option<&crate::provider_observation::ProviderObservationRecorder>,
) -> Result<()> {
    let Some(request) = rss_request_spec(context) else {
        return Ok(());
    };
    let feed_url = request.request_uri;
    let result = match recorder {
        Some(recorder) => rss::enrich_track_from_feed_rss_observed(context, &feed_url, recorder),
        None => rss::enrich_track_from_feed_rss(context, &feed_url),
    };
    crate::provider_observation::propagate_storage_failure(result)?;
    Ok(())
}

pub(crate) fn prepare_track_for_subscription_internal(
    cfg: &config::DownloadConfig,
    track: &Track,
    local_path: Option<&Path>,
) -> Result<PreparedTrack> {
    if let Some(path) = local_path {
        if path.exists() {
            return Ok(PreparedTrack::Existing {
                path: path.to_path_buf(),
            });
        }
    }
    if let Some(enclosure) = select_audio_enclosure(track) {
        let candidate = local_track_path(cfg, track, enclosure.format.canonical_extension());
        if candidate.exists() {
            return Ok(PreparedTrack::Existing { path: candidate });
        }
    }

    Ok(PreparedTrack::Downloaded(Box::new(download_track(
        cfg, track,
    )?)))
}

pub fn download_image(url: &str) -> Option<crate::metadata::ImageBytes> {
    let response = crate::remote_media::fetch(url, "cover art").ok()?;
    let declared = crate::remote_media::declared_content_type(&response);
    let bytes = response.bytes().ok()?.to_vec();
    if bytes.is_empty() {
        return None;
    }
    // Bytes first, declared type second. A non-image response is no image, not
    // an image with a bad label (ADR 0056).
    let mime_type = crate::media::image_type::classify(&bytes, declared.as_deref())?;
    Some(crate::metadata::ImageBytes {
        data: bytes,
        mime_type,
    })
}

pub fn compare_downloaded_track_path(
    path: &Path,
    track_context: &TrackContext,
) -> Result<TagCompareResult> {
    let mut track_context = track_context.clone();
    sanitize_track_context_source_text(&mut track_context);
    let tags = read_audio_tags(path)?;
    let file_image = tags.artwork.as_ref().and_then(|art| {
        if art.data.is_empty() {
            None
        } else {
            Some(crate::metadata::ImageBytes {
                data: art.data.clone(),
                mime_type: art.mime_type.clone(),
            })
        }
    });
    let track = &track_context.track;
    let mut rows = crate::metadata::compare_track_rows(track, track_context.feed.as_ref(), &tags);
    let detected = crate::audio_format::AudioFormat::detect_from_file(path).ok();
    if let Some(detected) = detected {
        crate::metadata::push_compare_row(
            &mut rows,
            "File format",
            None,
            Some(detected.display_label().to_string()),
        );
    }

    Ok(TagCompareResult {
        path: path.display().to_string(),
        rows,
        file_image,
        contributors: track.source_contributors.clone().unwrap_or_default(),
        value_routes: track.payment_routes.clone().unwrap_or_default(),
        id3_fields: tags.fields.clone(),
        total_tracks: tags.total_tracks.clone(),
        format: detected,
    })
}

fn authoritative_track_for_persistence<F>(
    explicit_track: Option<Track>,
    original_track: &Track,
    fetch_track: F,
) -> Option<Track>
where
    F: FnOnce(&str) -> Result<Track>,
{
    if let Some(mut explicit_track) = explicit_track {
        sanitize_track_source_text(&mut explicit_track);
        return Some(explicit_track);
    }

    let track_guid = original_track
        .track_guid
        .as_deref()
        .filter(|guid| !source_text_missing(Some(guid)))?;
    let mut fetched = fetch_track(track_guid).ok()?;
    fill_missing_track_match_fields(&mut fetched, original_track);
    sanitize_track_source_text(&mut fetched);
    Some(fetched)
}

fn fill_missing_track_match_fields(track: &mut Track, fallback: &Track) {
    fill_missing_source_text(&mut track.track_guid, &fallback.track_guid);
    fill_missing_source_text(&mut track.feed_guid, &fallback.feed_guid);
    fill_missing_source_text(&mut track.enclosure_url, &fallback.enclosure_url);
}

fn fill_missing_source_text(target: &mut Option<String>, fallback: &Option<String>) {
    if target
        .as_deref()
        .is_some_and(|value| !source_text_missing(Some(value)))
    {
        return;
    }

    *target = fallback
        .as_ref()
        .filter(|value| !source_text_missing(Some(value.as_str())))
        .cloned();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::SourceEnclosure;
    use crate::config::DownloadConfig;
    use crate::metadata_service::id3_edits_for_track_context;

    fn cfg(temp: &std::path::Path) -> DownloadConfig {
        DownloadConfig {
            music_dir: temp.join("music"),
            flac_path: Ok(None),
        }
    }

    fn track_with_enclosure(url: &str) -> Track {
        let mut t = Track {
            track_guid: Some("guid".into()),
            feed_guid: Some("feed".into()),
            feed_title: Some("Feed".into()),
            title: Some("Title".into()),
            track_number: Some(1),
            track_artist: Some("Artist".into()),
            ..Track::default()
        };
        t.source_enclosures = Some(vec![SourceEnclosure {
            url: Some(url.into()),
            mime_type: Some("audio/mpeg".into()),
            is_primary: Some(true),
            ..SourceEnclosure::default()
        }]);
        t
    }

    #[test]
    fn authoritative_persistence_prefers_explicit_track_payload() {
        let original = Track {
            track_guid: Some("track-guid".into()),
            publisher_text: Some("Feed Publisher".into()),
            description: Some("Feed description".into()),
            ..Track::default()
        };
        let explicit = Track {
            track_guid: Some("track-guid".into()),
            publisher_text: Some("Track Publisher".into()),
            description: Some("Track description".into()),
            ..Track::default()
        };

        let resolved =
            authoritative_track_for_persistence(Some(explicit), &original, |_| unreachable!())
                .expect("explicit track should resolve");

        assert_eq!(resolved.publisher_text.as_deref(), Some("Track Publisher"));
        assert_eq!(resolved.description.as_deref(), Some("Track description"));
    }

    #[test]
    fn authoritative_persistence_does_not_fallback_to_defaulted_context_on_fetch_error() {
        let original = Track {
            track_guid: Some("track-guid".into()),
            publisher_text: Some("Feed Publisher".into()),
            description: Some("Feed description".into()),
            ..Track::default()
        };

        let resolved = authoritative_track_for_persistence(None, &original, |_| {
            Err(anyhow::anyhow!("offline"))
        });

        assert!(
            resolved.is_none(),
            "defaulted context metadata must not be persisted as track-owned facts"
        );
    }

    #[test]
    fn fetched_authoritative_persistence_merges_only_match_fields() {
        let original = Track {
            track_guid: Some("track-guid".into()),
            feed_guid: Some("feed-guid".into()),
            enclosure_url: Some("https://example.test/audio.mp3".into()),
            publisher_text: Some("Feed Publisher".into()),
            description: Some("Feed description".into()),
            pub_date: Some(1_714_300_000),
            explicit: Some(true),
            ..Track::default()
        };
        let fetched = Track {
            track_guid: Some("track-guid".into()),
            publisher_text: Some("Track Publisher".into()),
            ..Track::default()
        };

        let resolved = authoritative_track_for_persistence(None, &original, |_| Ok(fetched))
            .expect("fetched track should resolve");

        assert_eq!(resolved.feed_guid.as_deref(), Some("feed-guid"));
        assert_eq!(
            resolved.enclosure_url.as_deref(),
            Some("https://example.test/audio.mp3")
        );
        assert_eq!(resolved.publisher_text.as_deref(), Some("Track Publisher"));
        assert_eq!(
            resolved.description, None,
            "feed-default description must not be copied into fetched persistence track"
        );
        assert_eq!(
            resolved.pub_date, None,
            "feed/download-context pubdate must not be copied into fetched persistence track"
        );
        assert_eq!(
            resolved.explicit, None,
            "feed/download-context explicit flag must not be copied into fetched persistence track"
        );
    }

    #[test]
    fn prepare_returns_existing_when_local_path_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let local = temp.path().join("existing.mp3");
        std::fs::write(&local, b"data").expect("write");
        let cfg = cfg(temp.path());
        let track = track_with_enclosure("https://nowhere.invalid/song.mp3");

        let prepared = prepare_track_for_subscription_internal(&cfg, &track, Some(local.as_path()))
            .expect("prepared");
        assert!(matches!(prepared, PreparedTrack::Existing { .. }));
    }

    #[test]
    fn prepare_returns_existing_when_candidate_in_music_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg = cfg(temp.path());
        let track = track_with_enclosure("https://nowhere.invalid/song.mp3");
        let candidate = local_track_path(&cfg, &track, "mp3");
        std::fs::create_dir_all(candidate.parent().unwrap()).expect("mkdir");
        std::fs::write(&candidate, b"data").expect("write");

        let prepared =
            prepare_track_for_subscription_internal(&cfg, &track, None).expect("prepared");
        assert!(matches!(prepared, PreparedTrack::Existing { .. }));
    }

    #[test]
    fn track_row_to_api_track_maps_identity_fields() {
        let row = TrackRow {
            id: 7,
            feed_id: 3,
            feed_guid: Some("fg".into()),
            item_guid: "ig".into(),
            track_title: Some("Title".into()),
            artist_name: Some("Artist".into()),
            album_title: None,
            album_artist_name: Some("Album Artist".into()),
            track_number: Some(2),
            disc_number: None,
            duration_seconds: Some(120),
            enclosure_url: Some("https://x/audio.mp3".into()),
            enclosure_type: Some("audio/mpeg".into()),
            track_image_href: Some("https://x/art.jpg".into()),
            is_in_library: false,
            feed_title: Some("Feed".into()),
            album_image_href: None,
            local_path: None,
            pub_date: Some(1_712_275_200),
            explicit: Some(true),
            transcript_url: Some("https://x/transcript.vtt".into()),
        };

        let api = track_row_to_api_track(&row);
        assert_eq!(api.track_guid.as_deref(), Some("ig"));
        assert_eq!(api.feed_guid.as_deref(), Some("fg"));
        assert_eq!(api.title.as_deref(), Some("Title"));
        assert_eq!(api.track_artist.as_deref(), Some("Artist"));
        assert_eq!(api.track_number, Some(2));
        assert_eq!(api.duration_secs, Some(120));
        assert_eq!(api.pub_date, Some(1_712_275_200));
        assert_eq!(api.explicit, Some(true));
        assert_eq!(api.enclosure_url.as_deref(), Some("https://x/audio.mp3"));
        let links = api.source_links.expect("source_links");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type.as_deref(), Some("transcript"));
        assert_eq!(links[0].url.as_deref(), Some("https://x/transcript.vtt"));
    }

    #[test]
    fn local_track_for_feed_download_uses_same_feed_rss_row_by_position() {
        let remote = Track {
            title: Some("Remote Title".into()),
            track_number: Some(4),
            ..Track::default()
        };
        let local = TrackRow {
            id: 7,
            item_guid: "rss-guid".into(),
            track_title: Some("RSS Title".into()),
            track_number: Some(4),
            enclosure_url: Some("https://x/audio.mp3".into()),
            enclosure_type: Some("audio/mpeg".into()),
            ..TrackRow::default()
        };

        let selected = local_track_for_feed_download(&remote, std::slice::from_ref(&local), 0, 1)
            .expect("same feed row");

        assert_eq!(selected.enclosure_url, local.enclosure_url);
    }

    #[test]
    fn fill_missing_download_source_from_local_row_preserves_remote_identity() {
        let mut remote = Track {
            track_guid: Some("musicindex-track".into()),
            title: Some("Remote Title".into()),
            ..Track::default()
        };
        let local = TrackRow {
            item_guid: "rss-guid".into(),
            track_title: Some("RSS Title".into()),
            track_number: Some(4),
            enclosure_url: Some("https://x/audio.mp3".into()),
            enclosure_type: Some("audio/mpeg".into()),
            track_image_href: Some("https://x/art.jpg".into()),
            feed_title: Some("Feed".into()),
            ..TrackRow::default()
        };

        fill_missing_download_source_from_local_row(&mut remote, &local);

        assert_eq!(remote.track_guid.as_deref(), Some("musicindex-track"));
        assert_eq!(remote.title.as_deref(), Some("Remote Title"));
        assert_eq!(remote.enclosure_url.as_deref(), Some("https://x/audio.mp3"));
        assert_eq!(remote.enclosure_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(remote.image_url.as_deref(), Some("https://x/art.jpg"));
        assert_eq!(remote.feed_title.as_deref(), Some("Feed"));
    }

    #[test]
    fn enrich_no_feed_url_is_noop() {
        let track = Track {
            title: Some("Title".into()),
            ..Track::default()
        };
        let mut context = TrackContext::new(track, None);
        let before = context.track.clone();
        enrich_track_context_from_rss(&mut context);
        assert_eq!(context.track.title, before.title);
    }

    /// A local HTTP server for the ADR 0076 task 009 tests. It gives each
    /// listed path its body, and 404 for each other path. No request goes
    /// to a remote host.
    struct FixtureServer {
        base: String,
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl FixtureServer {
        fn start(routes: impl Fn(&str, &str) -> Option<Vec<u8>> + Send + 'static) -> Self {
            use std::io::{Read, Write};
            use std::sync::atomic::{AtomicBool, Ordering};

            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            listener.set_nonblocking(true).unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = Arc::clone(&stop);
            let worker_base = base.clone();
            let worker = std::thread::spawn(move || {
                while !worker_stop.load(Ordering::Relaxed) {
                    let Ok((mut stream, _)) = listener.accept() else {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        continue;
                    };
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 4096];
                    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                        match stream.read(&mut buffer) {
                            Ok(0) | Err(_) => break,
                            Ok(read) => request.extend_from_slice(&buffer[..read]),
                        }
                    }
                    let text = String::from_utf8_lossy(&request);
                    let path = text.split_whitespace().nth(1).unwrap_or("/").to_owned();
                    let (status, body) = match routes(&worker_base, &path) {
                        Some(body) => ("200 OK", body),
                        None => ("404 Not Found", Vec::new()),
                    };
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(&body);
                }
            });
            Self {
                base,
                stop,
                worker: Some(worker),
            }
        }
    }

    impl Drop for FixtureServer {
        fn drop(&mut self) {
            self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    const CHANNEL_LINK: &str = "https://v4vmusic.com/?publisher=fixture-publisher";
    const MP3_BODY: &[u8] = b"ID3\x04\x00\x00\x00\x00\x00\x00mp3data";

    /// A parsed RSS fixture of two items. The channel has a `<link>`.
    fn rss_document(base: &str) -> Vec<u8> {
        let items = [(1, "Song One"), (2, "Song Two")]
            .iter()
            .map(|(number, title)| {
                format!(
                    r#"<item><title>{title}</title><guid isPermaLink="false">item-{number}</guid><description>Song notes</description><pubDate>Tue, 01 Sep 2026 10:00:00 +0000</pubDate><enclosure url="{base}/song-{number}.mp3" type="audio/mpeg" length="17"/><itunes:duration>3:05</itunes:duration><itunes:author>The Band</itunes:author><podcast:episode>{number}</podcast:episode><podcast:value type="lightning" method="keysend"><podcast:valueRecipient name="Band" type="node" address="03abcdef" split="100"/></podcast:value></item>"#
                )
            })
            .collect::<String>();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" xmlns:podcast="https://podcastindex.org/namespace/1.0"><channel><title>Album One</title><link>{CHANNEL_LINK}</link><description>Album notes</description><language>en</language><itunes:author>The Band</itunes:author><podcast:guid>feed-guid-1</podcast:guid><podcast:medium>music</podcast:medium><podcast:value type="lightning" method="keysend"><podcast:valueRecipient name="Band" type="node" address="03abcdef" split="100"/></podcast:value>{items}</channel></rss>"#
        )
        .into_bytes()
    }

    /// The request feed of a Index download. Its values equal the RSS
    /// values, and it has no `source_links`, as `api_feed_from_view` gives.
    fn request_feed(base: &str) -> Feed {
        let route = crate::api::PaymentRoute {
            recipient_name: Some("Band".into()),
            route_type: Some("node".into()),
            address: Some("03abcdef".into()),
            split: Some(100.0),
            fee: Some(false),
            ..crate::api::PaymentRoute::default()
        };
        let track = |number: i32, title: &str| Track {
            track_guid: Some(format!("item-{number}")),
            feed_guid: Some("feed-guid-1".into()),
            feed_title: Some("Album One".into()),
            title: Some(title.into()),
            track_number: Some(number),
            duration_secs: Some(185),
            description: Some("Song notes".into()),
            enclosure_url: Some(format!("{base}/song-{number}.mp3")),
            enclosure_type: Some("audio/mpeg".into()),
            track_artist: Some("The Band".into()),
            release_artist: Some("The Band".into()),
            payment_routes: Some(vec![route.clone()]),
            ..Track::default()
        };
        Feed {
            feed_guid: Some("feed-guid-1".into()),
            feed_url: Some(format!("{base}/feed.xml")),
            title: Some("Album One".into()),
            release_artist: Some("The Band".into()),
            language: Some("en".into()),
            description: Some("Album notes".into()),
            episode_count: Some(2),
            tracks: Some(vec![track(1, "Song One"), track(2, "Song Two")]),
            payment_routes: Some(vec![route.clone()]),
            ..Feed::default()
        }
    }

    /// Download the request feed of `request_feed` into a fresh database
    /// and music folder.
    fn download_fixture_feed() -> (
        tempfile::TempDir,
        Arc<Mutex<Connection>>,
        Result<SubscribeFeedOutcome>,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let server = FixtureServer::start(|base, path| match path {
            "/feed.xml" => Some(rss_document(base)),
            "/song-1.mp3" | "/song-2.mp3" => Some(MP3_BODY.to_vec()),
            _ => None,
        });
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::migrate_schema(&conn).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let outcome = subscribe_feed_retaining(
            Arc::clone(&conn),
            &cfg(temp.path()),
            SubscribeFeedRequest {
                feed: request_feed(&server.base),
                musicindex_endpoint: server.base.as_str().into(),
            },
            |_, _, _| Ok(()),
        );
        drop(server);
        (temp, conn, outcome)
    }

    /// The stored projection of each Library track: the frames that the
    /// scan expects, and then the stored route frame.
    fn stored_projection_edits(conn: &Connection) -> Vec<(TrackRow, Vec<Id3v24Edit>)> {
        db::library_tracks(conn)
            .unwrap()
            .into_iter()
            .map(|row| {
                let context =
                    crate::feed_service::track_row_to_track_context_with_local_identity(conn, &row)
                        .unwrap();
                let mut edits = id3_edits_for_track_context(&context);
                edits.retain(|edit| {
                    edit.frame_label != crate::metadata::MUSICINDEX_VALUE_ROUTES_FRAME
                });
                if let Some(value) = db::payment_routes::stored_route(conn, row.id)
                    .unwrap()
                    .as_deref()
                    .and_then(crate::metadata::summarize_value_routes)
                {
                    edits.push(Id3v24Edit {
                        frame_label: crate::metadata::MUSICINDEX_VALUE_ROUTES_FRAME.into(),
                        value,
                    });
                }
                (row, edits)
            })
            .collect()
    }

    fn file_values(path: &Path, frame_id: &str) -> Vec<String> {
        read_audio_tags(path)
            .unwrap()
            .fields
            .into_iter()
            .filter(|field| field.frame_id == frame_id)
            .map(|field| field.value)
            .collect()
    }

    /// R76-9-01: a download from a request feed with no `source_links`
    /// writes `WOAR` with the channel `<link>` of the RSS document.
    #[test]
    fn adr_0076_download_stored_values_r76_9_01_download_writes_channel_link() {
        let (temp, conn, outcome) = download_fixture_feed();
        assert_eq!(outcome.unwrap().downloaded, 2);
        let music_dir = cfg(temp.path()).music_dir;
        let rows = db::library_tracks(&conn.lock().unwrap()).unwrap();
        assert_eq!(rows.len(), 2);
        for row in rows {
            let path = row.local_path.unwrap().resolve(&music_dir);
            assert_eq!(
                file_values(&path, "WOAR"),
                vec![CHANNEL_LINK.to_owned()],
                "the download must write the channel link to {}",
                path.display()
            );
        }
    }

    /// R76-9-02: after the download of R76-9-01, the scan gives no changed
    /// frame for each new file.
    #[test]
    fn adr_0076_download_stored_values_r76_9_02_scan_after_download_is_empty() {
        let (temp, conn, outcome) = download_fixture_feed();
        outcome.unwrap();
        let scan = crate::application::queries::tag_update::scan_tag_updates(
            &conn.lock().unwrap(),
            &cfg(temp.path()).music_dir,
        )
        .unwrap();
        assert_eq!(
            scan.count(),
            0,
            "a fresh download must not differ: {:?}",
            scan.files
        );
    }

    /// R76-9-03: the edits that the download writes equal the edits that
    /// the scan expects. The writer writes each edit as one value, so the
    /// count and the file values prove the equality.
    #[test]
    fn adr_0076_download_stored_values_r76_9_03_download_edits_equal_scan_edits() {
        let (temp, conn, outcome) = download_fixture_feed();
        let outcome = outcome.unwrap();
        let music_dir = cfg(temp.path()).music_dir;
        let db = conn.lock().unwrap();
        let expected = stored_projection_edits(&db);
        assert_eq!(expected.len(), 2);
        assert_eq!(
            outcome.applied_edits,
            expected.iter().map(|(_, edits)| edits.len()).sum::<usize>(),
            "the download must write each expected edit and no other edit"
        );
        for (row, edits) in expected {
            let path = row.local_path.as_ref().unwrap().resolve(&music_dir);
            for edit in &edits {
                // The scan compares the route frame as routes, not as text.
                if edit.frame_label == crate::metadata::MUSICINDEX_VALUE_ROUTES_FRAME {
                    continue;
                }
                let values = read_audio_tags(&path)
                    .unwrap()
                    .fields
                    .into_iter()
                    .filter(|field| {
                        crate::metadata::pending_id3_target_key(&field.frame_id)
                            == crate::metadata::pending_id3_target_key(&edit.frame_label)
                    })
                    .map(|field| field.value)
                    .collect::<Vec<_>>();
                assert!(
                    values.iter().any(|value| value.trim() == edit.value.trim()),
                    "{} must hold {:?}, file values {values:?}",
                    edit.frame_label,
                    edit.value
                );
            }
        }
    }

    /// R46-04: RSS enrichment of a context with a feed address reads the
    /// feed address from the context's feed. `api::Track` has no `feed_url`
    /// field of its own (ADR 0075 packet 046).
    #[test]
    fn adr_0075_undeclared_fields_r46_04_rss_request_spec_reads_feed_address_from_feed() {
        let context = TrackContext::new(
            Track {
                track_guid: Some("track-1".into()),
                ..Track::default()
            },
            Some(Feed {
                feed_url: Some("https://example.test/feed.xml".into()),
                ..Feed::default()
            }),
        );

        let spec = rss_request_spec(&context).expect("a context with a feed address requests RSS");
        assert_eq!(spec.request_uri, "https://example.test/feed.xml");
    }
}
