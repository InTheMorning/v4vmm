use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use reqwest::blocking::Client as ReqwestClient;
use rusqlite::Connection;

use crate::api::{track_with_feed_defaults, Client, Feed, SourceEntityLink, Track};
use crate::audio_tags::{read_audio_tags, write_id3v24_edits, Id3v24Edit};
use crate::config;
use crate::db::{self, TrackRow};
use crate::identity_ingest;
use crate::library_service;
use crate::metadata::{MusicBrainzLookupResult, TagCompareResult, TrackContext};
use crate::metadata_service::{id3_edits_for_track_context, musicbrainz_lookup_metadata};
use crate::musicbrainz::lookup_recordings;
use crate::rss;
use crate::track_compare::{download_track, local_track_path, select_audio_enclosure};

pub enum SubscribeTrackRequest {
    LibraryTrack {
        track: Box<TrackRow>,
    },
    SearchTrack {
        track_context: Box<TrackContext>,
        edits: Vec<Id3v24Edit>,
        musicindex_endpoint: String,
        mark_feed_subscribed: bool,
        return_tag_compare: bool,
    },
}

pub struct SubscribeTrackOutcome {
    pub path: PathBuf,
    pub format_warning: Option<String>,
    pub applied_edits: usize,
    pub marked_downloaded: bool,
    pub compare: Option<TagCompareResult>,
}

pub struct SubscribeFeedRequest {
    pub feed: Feed,
    pub musicindex_endpoint: String,
}

pub struct SubscribeFeedOutcome {
    pub downloaded: usize,
    pub applied_edits: usize,
    pub skipped: usize,
}

pub(crate) enum PreparedTrack {
    Existing { path: PathBuf },
    Downloaded(crate::track_compare::DownloadedTrack),
}

impl PreparedTrack {
    fn working_path(&self) -> &Path {
        match self {
            PreparedTrack::Existing { path } => path.as_path(),
            PreparedTrack::Downloaded(d) => d.path.as_path(),
        }
    }

    fn finalize(self) -> Result<PathBuf> {
        match self {
            PreparedTrack::Existing { path } => Ok(path),
            PreparedTrack::Downloaded(d) => d.finalize(),
        }
    }

    fn format_warning(&self) -> Option<String> {
        match self {
            PreparedTrack::Existing { .. } => None,
            PreparedTrack::Downloaded(d) => d.format_warning.clone(),
        }
    }
}

pub fn subscribe_track(
    conn: Arc<Mutex<Connection>>,
    request: SubscribeTrackRequest,
) -> Result<SubscribeTrackOutcome> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    subscribe_track_with_config(conn, &cfg, request)
}

pub(crate) fn subscribe_track_with_config(
    conn: Arc<Mutex<Connection>>,
    cfg: &config::Config,
    request: SubscribeTrackRequest,
) -> Result<SubscribeTrackOutcome> {
    match request {
        SubscribeTrackRequest::LibraryTrack { track } => {
            subscribe_library_track_internal(conn, cfg, *track)
        }
        SubscribeTrackRequest::SearchTrack {
            track_context,
            edits,
            musicindex_endpoint,
            mark_feed_subscribed,
            return_tag_compare,
        } => subscribe_track_from_search_internal(
            conn,
            cfg,
            *track_context,
            edits,
            musicindex_endpoint,
            mark_feed_subscribed,
            return_tag_compare,
        ),
    }
}

pub fn subscribe_feed(
    conn: Arc<Mutex<Connection>>,
    request: SubscribeFeedRequest,
) -> Result<SubscribeFeedOutcome> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    subscribe_feed_with_config(conn, &cfg, request)
}

pub(crate) fn subscribe_feed_with_config(
    conn: Arc<Mutex<Connection>>,
    cfg: &config::Config,
    request: SubscribeFeedRequest,
) -> Result<SubscribeFeedOutcome> {
    let feed = request.feed;
    let musicindex_endpoint = request.musicindex_endpoint;
    let feed_url = feed
        .feed_url
        .clone()
        .ok_or_else(|| anyhow!("feed has no RSS URL"))?;

    {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        rss::subscribe_feed(cfg, &mut db, &feed_url, &musicindex_endpoint)?;
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
    let tracks = feed.tracks.clone().unwrap_or_default();
    let track_count = tracks.len();

    for track in tracks {
        let original_track = track;
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
            }
        }
        {
            let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
            identity_ingest::persist_musicindex_context_by_feed_url(
                &mut db,
                &feed_url,
                None,
                Some(&track_for_persistence),
            )?;
        }
        let mut track = track_with_feed_defaults(track_for_metadata, Some(&feed));
        let mut context_feed = feed.clone();
        enrich_track_context_from_rss(&mut track, Some(&mut context_feed));
        let track_context = TrackContext {
            track: track.clone(),
            feed: Some(context_feed),
        };
        let edits = id3_edits_for_track_context(&track_context);

        match subscribe_track_from_search_internal(
            Arc::clone(&conn),
            cfg,
            track_context,
            edits.clone(),
            musicindex_endpoint.clone(),
            true,  // mark_feed_subscribed
            false, // return_tag_compare
        ) {
            Ok(outcome) => {
                if outcome.marked_downloaded {
                    downloaded += 1;
                } else {
                    skipped += 1;
                }
                applied_edits += outcome.applied_edits;
            }
            Err(err) => {
                eprintln!(
                    "skip {}: {err:#}",
                    track.title.as_deref().unwrap_or("(untitled)")
                );
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

fn subscribe_library_track_internal(
    conn: Arc<Mutex<Connection>>,
    cfg: &config::Config,
    track: TrackRow,
) -> Result<SubscribeTrackOutcome> {
    let api_track = track_row_to_api_track(&track);

    let prepared = prepare_track_for_subscription_internal(
        cfg,
        &ReqwestClient::new(),
        &api_track,
        track.local_path.as_deref(),
    )?;

    let track_context = TrackContext {
        track: api_track,
        feed: None,
    };
    let edits = id3_edits_for_track_context(&track_context);
    let format_warning = prepared.format_warning();
    let working_path = prepared.working_path().to_path_buf();
    let applied_edits = apply_id3_edits_nonfatal(&working_path, &edits);

    let final_path = prepared.finalize()?;
    let file_size = std::fs::metadata(&final_path)
        .ok()
        .and_then(|metadata| metadata.len().try_into().ok());
    let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    library_service::mark_track_downloaded(&db, track.id, &final_path, file_size)?;

    Ok(SubscribeTrackOutcome {
        path: final_path,
        format_warning,
        applied_edits,
        marked_downloaded: true,
        compare: None,
    })
}

fn subscribe_track_from_search_internal(
    conn: Arc<Mutex<Connection>>,
    cfg: &config::Config,
    track_context: TrackContext,
    edits: Vec<Id3v24Edit>,
    musicindex_endpoint: String,
    mark_feed_subscribed: bool,
    return_tag_compare: bool,
) -> Result<SubscribeTrackOutcome> {
    let mut feed = track_context.feed;
    let original_track = track_context.track.clone();
    let mut track = track_with_feed_defaults(original_track.clone(), feed.as_ref());
    enrich_track_context_from_rss(&mut track, feed.as_mut());
    let feed_url = track
        .feed_url
        .clone()
        .or_else(|| feed.as_ref().and_then(|feed| feed.feed_url.clone()))
        .ok_or_else(|| anyhow!("track has no RSS feed URL"))?;

    let prior_subscribed = {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        db::feed_is_subscribed_by_url(&db, &feed_url).unwrap_or(false)
    };

    {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        rss::subscribe_feed(cfg, &mut db, &feed_url, &musicindex_endpoint)?;
        identity_ingest::persist_musicindex_context_by_feed_url(
            &mut db,
            &feed_url,
            feed.as_ref(),
            Some(&original_track),
        )?;
        if !mark_feed_subscribed && !prior_subscribed {
            db::set_feed_subscribed_by_url(&db, &feed_url, false)?;
        }
    }

    let client = ReqwestClient::new();
    let prepared = prepare_track_for_subscription_internal(cfg, &client, &track, None)
        .inspect_err(|_| {
            if mark_feed_subscribed && !prior_subscribed {
                if let Ok(db) = conn.lock() {
                    let _ = db::set_feed_subscribed_by_url(&db, &feed_url, false);
                }
            }
        })?;

    let working_path = prepared.working_path().to_path_buf();
    let applied_edits = apply_id3_edits_nonfatal(&working_path, &edits);
    let format_warning = prepared.format_warning();
    let path = prepared.finalize()?;
    let file_size = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.len().try_into().ok());

    let marked_downloaded = {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        let marked_downloaded = library_service::mark_track_downloaded_by_match(
            &db,
            Some(feed_url.as_str()),
            track.track_guid.as_deref(),
            track.enclosure_url.as_deref(),
            &path,
            file_size,
        )?;
        if !mark_feed_subscribed {
            db::reconcile_feed_subscription_by_url(&db, &feed_url)?;
        }
        marked_downloaded
    };

    let compare = if return_tag_compare {
        let refreshed_context = TrackContext { track, feed };
        Some(compare_downloaded_track_path(&path, &refreshed_context)?)
    } else {
        None
    };

    Ok(SubscribeTrackOutcome {
        path,
        format_warning,
        applied_edits,
        marked_downloaded,
        compare,
    })
}

fn apply_id3_edits_nonfatal(path: &Path, edits: &[Id3v24Edit]) -> usize {
    if edits.is_empty() {
        return 0;
    }
    match write_id3v24_edits(path, edits) {
        Ok(_) => edits.len(),
        Err(err) => {
            eprintln!("skip tag write for {}: {err:#}", path.display());
            0
        }
    }
}

pub fn track_row_to_api_track(row: &TrackRow) -> Track {
    Track {
        track_guid: Some(row.item_guid.clone()),
        feed_guid: row.feed_guid.clone(),
        title: row.track_title.clone(),
        track_artist: row.artist_name.clone(),
        release_artist: row.album_artist_name.clone(),
        feed_title: row.feed_title.clone(),
        track_number: row.track_number.and_then(|n| n.try_into().ok()),
        duration_secs: row.duration_seconds.and_then(|s| s.try_into().ok()),
        enclosure_url: row.enclosure_url.clone(),
        enclosure_type: row.enclosure_type.clone(),
        image_url: row.track_image_href.clone(),
        publisher_text: None,
        description: None,
        source_links: row.transcript_url.as_ref().map(|url| {
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

pub fn enrich_track_context_from_rss(track: &mut Track, feed: Option<&mut Feed>) {
    let feed_url = track
        .feed_url
        .clone()
        .or_else(|| feed.as_ref().and_then(|feed| feed.feed_url.clone()));
    let Some(feed_url) = feed_url else {
        return;
    };
    let _ = rss::enrich_track_from_feed_rss(track, feed, &feed_url);
}

pub(crate) fn prepare_track_for_subscription_internal(
    cfg: &config::Config,
    client: &ReqwestClient,
    track: &Track,
    local_path: Option<&str>,
) -> Result<PreparedTrack> {
    if let Some(path_str) = local_path {
        let buf = PathBuf::from(path_str);
        if buf.exists() {
            return Ok(PreparedTrack::Existing {
                path: crate::track_compare::ensure_taggable_local_path(cfg, &buf),
            });
        }
    }
    if let Some(enclosure) = select_audio_enclosure(track) {
        let candidate = local_track_path(cfg, track, enclosure.format.canonical_extension());
        if candidate.exists() {
            return Ok(PreparedTrack::Existing {
                path: crate::track_compare::ensure_taggable_local_path(cfg, &candidate),
            });
        }
    }

    Ok(PreparedTrack::Downloaded(download_track(
        cfg, client, track,
    )?))
}

pub fn download_and_compare_track(
    client: &Client,
    entity_id: &str,
    force_download: bool,
) -> Result<TagCompareResult> {
    let mut track = client.fetch_track(
        entity_id,
        Some(
            "source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes",
        ),
    )?;
    let mut feed = match track.feed_guid.as_deref() {
        Some(feed_guid) => client
            .fetch_feed(
                feed_guid,
                Some("tracks,source_enclosures,source_links,source_ids,source_release_claims"),
            )
            .ok(),
        None => None,
    };
    enrich_track_context_from_rss(&mut track, feed.as_mut());
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    if !force_download {
        if let Some(enclosure) = select_audio_enclosure(&track) {
            let candidate = local_track_path(&cfg, &track, enclosure.format.canonical_extension());
            if candidate.exists() {
                let path = crate::track_compare::ensure_taggable_local_path(&cfg, &candidate);
                let track_context = TrackContext { track, feed };
                return compare_downloaded_track_path(&path, &track_context);
            }
        }
    }
    let downloaded = download_track(&cfg, &client.client, &track)?;
    let track_context = TrackContext { track, feed };
    compare_downloaded_track_path(&downloaded.path, &track_context)
}

pub fn lookup_musicbrainz_track(
    client: &Client,
    entity_id: &str,
) -> Result<MusicBrainzLookupResult> {
    let track = client.fetch_track(entity_id, Some("source_enclosures"))?;
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let downloaded = download_track(&cfg, &client.client, &track)?;
    let tags = read_audio_tags(&downloaded.path)?;
    let metadata = musicbrainz_lookup_metadata(&track, &tags);
    let musicbrainz_client = ReqwestClient::builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&musicbrainz_client, &metadata, 5)?;
    let image = lookup
        .candidates
        .first()
        .and_then(|candidate| candidate.release_id.as_deref())
        .and_then(|release_id| {
            let url = format!("https://coverartarchive.org/release/{release_id}/front-250");
            download_image(&musicbrainz_client, &url)
        });
    Ok(MusicBrainzLookupResult { lookup, image })
}

pub fn download_image(client: &ReqwestClient, url: &str) -> Option<crate::metadata::ImageBytes> {
    let response = client.get(url).send().ok()?.error_for_status().ok()?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = response.bytes().ok()?.to_vec();
    if bytes.is_empty() {
        return None;
    }
    Some(crate::metadata::ImageBytes {
        data: bytes,
        mime_type: content_type,
    })
}

pub fn compare_downloaded_track_path(
    path: &Path,
    track_context: &TrackContext,
) -> Result<TagCompareResult> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::SourceEnclosure;
    use crate::config::{Config, PlaybackConfig};
    use crate::theme_profile::ThemeProfile;

    fn cfg(temp: &std::path::Path) -> Config {
        Config {
            music_dir: temp.join("music"),
            db_path: temp.join("db.sqlite"),
            flac_path: None,
            playback: PlaybackConfig::default(),
            ui_scale: Default::default(),
            theme_profile: ThemeProfile::default(),
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
    fn prepare_returns_existing_when_local_path_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let local = temp.path().join("existing.mp3");
        std::fs::write(&local, b"data").expect("write");
        let cfg = cfg(temp.path());
        let track = track_with_enclosure("https://nowhere.invalid/song.mp3");

        let prepared = prepare_track_for_subscription_internal(
            &cfg,
            &ReqwestClient::new(),
            &track,
            local.to_str(),
        )
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
            prepare_track_for_subscription_internal(&cfg, &ReqwestClient::new(), &track, None)
                .expect("prepared");
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
            transcript_url: Some("https://x/transcript.vtt".into()),
        };

        let api = track_row_to_api_track(&row);
        assert_eq!(api.track_guid.as_deref(), Some("ig"));
        assert_eq!(api.feed_guid.as_deref(), Some("fg"));
        assert_eq!(api.title.as_deref(), Some("Title"));
        assert_eq!(api.track_artist.as_deref(), Some("Artist"));
        assert_eq!(api.track_number, Some(2));
        assert_eq!(api.duration_secs, Some(120));
        assert_eq!(api.enclosure_url.as_deref(), Some("https://x/audio.mp3"));
        let links = api.source_links.expect("source_links");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type.as_deref(), Some("transcript"));
        assert_eq!(links[0].url.as_deref(), Some("https://x/transcript.vtt"));
    }

    #[test]
    fn enrich_no_feed_url_is_noop() {
        let mut track = Track {
            title: Some("Title".into()),
            ..Track::default()
        };
        let before = track.clone();
        enrich_track_context_from_rss(&mut track, None);
        assert_eq!(track.title, before.title);
        assert_eq!(track.feed_url, before.feed_url);
    }
}
