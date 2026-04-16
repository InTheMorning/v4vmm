use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use gpui::{
    div, img, prelude::*, px, rgb, AnyElement, Context, FontWeight, Image, ImageFormat,
    IntoElement, ObjectFit, Render, SharedString, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::Disableable;
use gpui_component::Sizable;
use gpui_component::Size;
use reqwest::blocking::Client as ReqwestClient;

use crate::api::{SourceEntityLink, Track};
use crate::audio_tags::{read_audio_tags, write_id3v24_edits, Id3v24Edit};
use crate::config;
use crate::db::{self, TrackRow};
use crate::metadata::TrackContext;
use crate::musicbrainz::{
    lookup_recordings, lookup_releases, LookupMetadata, MusicBrainzCandidate,
};
use crate::search::id3_edits_for_track_context;
use crate::track_compare::{download_track_mp3, local_mp3_path};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryTab {
    Library,
    Cached,
}

#[derive(Clone, Debug)]
enum LibraryDetail {
    None,
    Album(AlbumNode),
    Track(Box<TrackRow>),
}

#[derive(Clone, Debug)]
enum MbTrackStatus {
    Pending,
    Processing,
    Done(usize),
    Skipped(String),
}

#[derive(Clone, Debug)]
struct ArtistNode {
    name: String,
    albums: Vec<AlbumNode>,
}

#[derive(Clone, Debug)]
struct AlbumNode {
    name: String,
    feed_id: Option<i64>,
    image_href: Option<String>,
    tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug, Default)]
struct LibraryTree {
    artists: Vec<ArtistNode>,
}

#[derive(Clone)]
enum ThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
}

pub struct LibraryApp {
    conn: Arc<Mutex<Connection>>,
    tab: LibraryTab,
    tree: LibraryTree,
    cached_tree: LibraryTree,
    expanded_artists: HashSet<String>,
    expanded_albums: HashSet<(String, String)>,
    selected_id: Option<i64>,
    detail: LibraryDetail,
    status: String,
    busy_track: Option<i64>,
    mb_status: BTreeMap<i64, MbTrackStatus>,
    thumbnails: BTreeMap<String, ThumbnailState>,
}

// ---------------------------------------------------------------------------
// Color helpers (same palette as search.rs)
// ---------------------------------------------------------------------------

fn bg() -> gpui::Rgba {
    rgb(0x0f1117)
}
fn surface() -> gpui::Rgba {
    rgb(0x1a1d27)
}
fn border() -> gpui::Rgba {
    rgb(0x2a2d3a)
}
fn text() -> gpui::Rgba {
    rgb(0xe2e4ed)
}
fn muted() -> gpui::Rgba {
    rgb(0x9298ab)
}
fn accent() -> gpui::Rgba {
    rgb(0x8b9bff)
}

// ---------------------------------------------------------------------------
// LibraryApp
// ---------------------------------------------------------------------------

impl LibraryApp {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        let mut app = Self {
            conn,
            tab: LibraryTab::Library,
            tree: LibraryTree::default(),
            cached_tree: LibraryTree::default(),
            expanded_artists: HashSet::new(),
            expanded_albums: HashSet::new(),
            selected_id: None,
            detail: LibraryDetail::None,
            status: String::new(),
            busy_track: None,
            mb_status: BTreeMap::new(),
            thumbnails: BTreeMap::new(),
        };
        app.reload();
        app
    }

    fn reload(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match self.tab {
            LibraryTab::Library => match db::library_tracks(&conn) {
                Ok(rows) => {
                    let count = rows.len();
                    self.tree = build_tree(&rows);
                    self.status =
                        format!("{count} library track{}", if count == 1 { "" } else { "s" });
                }
                Err(err) => {
                    self.status = format!("Error: {err:#}");
                }
            },
            LibraryTab::Cached => match db::cached_tracks(&conn) {
                Ok(rows) => {
                    let count = rows.len();
                    self.cached_tree = build_tree(&rows);
                    self.status =
                        format!("{count} cached file{}", if count == 1 { "" } else { "s" });
                }
                Err(err) => {
                    self.status = format!("Error: {err:#}");
                }
            },
        }
        self.selected_id = None;
        self.detail = LibraryDetail::None;
        self.mb_status.clear();
    }

    fn list_is_empty(&self) -> bool {
        match self.tab {
            LibraryTab::Library => self.tree.artists.is_empty(),
            LibraryTab::Cached => self.cached_tree.artists.is_empty(),
        }
    }

    fn thumbnail_for_url(
        &mut self,
        url: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Image>> {
        let url = url?.trim();
        if url.is_empty() {
            return None;
        }
        match self.thumbnails.get(url) {
            Some(ThumbnailState::Loaded(image)) => return image.clone(),
            Some(ThumbnailState::Loading) => return None,
            None => {}
        }
        self.thumbnails
            .insert(url.to_string(), ThumbnailState::Loading);
        let url = url.to_string();
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let cache_url = url.clone();
                let image = cx
                    .background_executor()
                    .spawn(async move { load_thumbnail_image(&cache_url).ok().flatten() })
                    .await;
                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        this.thumbnails.insert(url, ThumbnailState::Loaded(image));
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
        None
    }

    fn select_album(&mut self, album: &AlbumNode) {
        self.selected_id = album.feed_id;
        self.detail = LibraryDetail::Album(album.clone());
    }

    fn select_track(&mut self, track: &TrackRow) {
        self.selected_id = Some(track.id);
        self.detail = LibraryDetail::Track(Box::new(track.clone()));
    }

    fn toggle_artist(&mut self, name: &str) {
        if !self.expanded_artists.remove(name) {
            self.expanded_artists.insert(name.to_string());
        }
    }

    fn toggle_album(&mut self, artist: &str, album: &str) {
        let key = (artist.to_string(), album.to_string());
        if !self.expanded_albums.remove(&key) {
            self.expanded_albums.insert(key);
        }
    }

    fn unsubscribe_feed(&mut self, feed_id: i64) {
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::set_feed_subscribed(&conn, feed_id, false) {
            self.status = format!("Error: {err:#}");
            return;
        }
        if let Err(err) = db::unsubscribe_feed_tracks(&conn, feed_id) {
            self.status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        self.reload();
    }

    fn remove_track(&mut self, track_id: i64) {
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::set_track_in_library(&conn, track_id, false) {
            self.status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        self.reload();
    }

    fn subscribe_track(&mut self, track: TrackRow, cx: &mut Context<Self>) {
        if self.busy_track.is_some() {
            return;
        }
        let track_id = track.id;
        self.busy_track = Some(track_id);
        self.status = "Subscribing track...".into();
        cx.notify();

        let conn = Arc::clone(&self.conn);
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { subscribe_library_track(conn, track) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        this.busy_track = None;
                        match result {
                            Ok(path) => {
                                this.status = format!("Subscribed track: {}", path.display());
                                this.reload();
                            }
                            Err(error) => {
                                this.status = format!("Error subscribing track: {error:#}");
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn delete_cached_file(&mut self, path: String) {
        if let Err(err) = std::fs::remove_file(&path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                self.status = format!("Error deleting file: {err:#}");
                return;
            }
        }
        cleanup_empty_parents(std::path::Path::new(&path));
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::delete_local_file(&conn, &path) {
            self.status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        self.reload();
    }

    fn delete_all_cached(&mut self) {
        let paths: Vec<String> = self
            .cached_tree
            .artists
            .iter()
            .flat_map(|a| &a.albums)
            .flat_map(|a| &a.tracks)
            .filter_map(|t| t.local_path.clone())
            .collect();
        for path in &paths {
            if let Err(err) = std::fs::remove_file(path) {
                if err.kind() != std::io::ErrorKind::NotFound {
                    self.status = format!("Error deleting {path}: {err:#}");
                    return;
                }
            }
            cleanup_empty_parents(std::path::Path::new(path));
        }
        let conn = self.conn.lock().expect("lock db");
        for path in &paths {
            if let Err(err) = db::delete_local_file(&conn, path) {
                self.status = format!("Error: {err:#}");
                return;
            }
        }
        drop(conn);
        self.reload();
    }

    fn musicbrainz_track(&mut self, track: TrackRow, cx: &mut Context<Self>) {
        if self.mb_status.contains_key(&track.id) {
            return;
        }
        self.mb_status.insert(track.id, MbTrackStatus::Processing);
        self.status = "MusicBrainz lookup...".into();
        cx.notify();

        let track_id = track.id;
        let conn = Arc::clone(&self.conn);
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { musicbrainz_autotag_track(conn, &track) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        match result {
                            Ok(n) => {
                                this.mb_status.insert(track_id, MbTrackStatus::Done(n));
                                this.status = format!(
                                    "MusicBrainz: applied {n} edit{}",
                                    if n == 1 { "" } else { "s" }
                                );
                            }
                            Err(err) => {
                                this.mb_status
                                    .insert(track_id, MbTrackStatus::Skipped(format!("{err:#}")));
                                this.status = format!("MusicBrainz error: {err:#}");
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn musicbrainz_feed(&mut self, album: AlbumNode, cx: &mut Context<Self>) {
        let downloadable: Vec<TrackRow> = album
            .tracks
            .into_iter()
            .filter(|t| t.local_path.is_some())
            .collect();
        if downloadable.is_empty() {
            self.status = "No downloaded tracks to process".into();
            cx.notify();
            return;
        }
        for t in &downloadable {
            self.mb_status.insert(t.id, MbTrackStatus::Pending);
        }
        self.status = format!(
            "MusicBrainz: album lookup for {} tracks...",
            downloadable.len()
        );
        cx.notify();

        let conn = Arc::clone(&self.conn);
        let feed_id = album.feed_id.unwrap_or(0);
        let feed_title = Some(album.name.clone());
        let total_count = downloadable.len();
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                // Build album-level metadata from feed + first track.
                let first_artist = downloadable.iter().find_map(|t| t.artist_name.clone());
                let album_metadata = LookupMetadata {
                    title: None,
                    artist: first_artist,
                    album: feed_title,
                    track_number: None,
                    total_tracks: Some(total_count.to_string()),
                    duration_secs: None,
                    isrc: None,
                };

                // Do album-level release search (blocking, on background thread).
                let meta_clone = album_metadata.clone();
                let release_candidates = cx
                    .background_executor()
                    .spawn(async move {
                        let mb_client = ReqwestClient::builder()
                            .user_agent(format!(
                                "v4vmm/{} (MusicBrainz metadata lookup)",
                                env!("CARGO_PKG_VERSION")
                            ))
                            .build()?;
                        lookup_releases(&mb_client, &meta_clone, 3)
                    })
                    .await;

                let candidates = match release_candidates {
                    Ok(c) => c,
                    Err(err) => {
                        // Fall back to per-track recording search.
                        this.update(
                            cx,
                            move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                                this.status = format!(
                                    "Album lookup failed ({err:#}), falling back to per-track..."
                                );
                                cx.notify();
                            },
                        )
                        .ok();
                        musicbrainz_feed_per_track(
                            this,
                            cx,
                            &conn,
                            &downloadable,
                            feed_id,
                            total_count,
                        )
                        .await;
                        return;
                    }
                };

                if candidates.is_empty() {
                    this.update(
                        cx,
                        move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                            this.status =
                                "Album lookup: no results, falling back to per-track...".into();
                            cx.notify();
                        },
                    )
                    .ok();
                    musicbrainz_feed_per_track(
                        this,
                        cx,
                        &conn,
                        &downloadable,
                        feed_id,
                        total_count,
                    )
                    .await;
                    return;
                }

                // Match each local track to best candidate by track position then title.
                let mut total_edits = 0usize;
                let mut processed = 0usize;
                for track in &downloadable {
                    let track_id = track.id;
                    let progress = processed + 1;
                    this.update(
                        cx,
                        move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                            this.mb_status.insert(track_id, MbTrackStatus::Processing);
                            this.status = format!(
                                "MusicBrainz: applying to track {progress}/{total_count} ...",
                            );
                            cx.notify();
                        },
                    )
                    .ok();

                    let matched = match_candidate_to_track(&candidates, track);
                    let track2 = track.clone();
                    let result = match matched {
                        Some(candidate) => {
                            let candidate = candidate.clone();
                            cx.background_executor()
                                .spawn(async move { apply_candidate_to_track(&track2, &candidate) })
                                .await
                        }
                        None => {
                            // No matching candidate — fall back to recording search for this track.
                            let conn2 = Arc::clone(&conn);
                            cx.background_executor()
                                .spawn(async move { musicbrainz_autotag_track(conn2, &track2) })
                                .await
                        }
                    };

                    let status = match result {
                        Ok(n) => {
                            total_edits += n;
                            MbTrackStatus::Done(n)
                        }
                        Err(err) => MbTrackStatus::Skipped(format!("{err:#}")),
                    };
                    processed += 1;

                    let status_clone = status.clone();
                    this.update(
                        cx,
                        move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                            this.mb_status.insert(track_id, status_clone);
                            cx.notify();
                        },
                    )
                    .ok();
                }

                // Refresh — rebuild tree and album detail
                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        this.status = format!(
                            "MusicBrainz: {total_edits} edit{} across {} tracks",
                            if total_edits == 1 { "" } else { "s" },
                            processed,
                        );
                        this.reload();
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }
}

fn subscribe_library_track(
    conn: Arc<Mutex<Connection>>,
    track: TrackRow,
) -> anyhow::Result<std::path::PathBuf> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let api_track = track_row_to_api_track(&track);
    let path = local_mp3_path(&cfg, &api_track);
    if !path.exists() {
        download_track_mp3(&cfg, &ReqwestClient::new(), &api_track)?;
    }
    let file_size = std::fs::metadata(&path)
        .ok()
        .and_then(|metadata| metadata.len().try_into().ok());
    let db = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
    db::mark_track_downloaded(&db, track.id, &path, file_size)?;
    drop(db);

    // Apply ID3 edits from RSS/musicindex metadata
    let track_context = TrackContext {
        track: api_track,
        feed: None,
    };
    let edits = id3_edits_for_track_context(&track_context);
    if !edits.is_empty() {
        write_id3v24_edits(&path, &edits)?;
    }
    Ok(path)
}

fn musicbrainz_autotag_track(
    _conn: Arc<Mutex<Connection>>,
    track: &TrackRow,
) -> anyhow::Result<usize> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("no local file"))?;
    let tags = read_audio_tags(std::path::Path::new(path))?;

    let api_track = track_row_to_api_track(track);
    let metadata = LookupMetadata {
        title: tags.title.clone().or_else(|| api_track.title.clone()),
        artist: tags
            .artist
            .clone()
            .or_else(|| api_track.track_artist.clone()),
        album: tags.album.clone().or_else(|| api_track.feed_title.clone()),
        track_number: tags
            .track_number
            .clone()
            .or_else(|| api_track.track_number.map(|n| n.to_string())),
        total_tracks: None,
        duration_secs: api_track.duration_secs.map(i64::from),
        isrc: tags
            .custom
            .get("ISRC")
            .cloned()
            .or_else(|| tags.custom.get("isrc").cloned()),
    };

    let mb_client = ReqwestClient::builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&mb_client, &metadata, 3)?;
    let candidate = lookup
        .candidates
        .first()
        .ok_or_else(|| anyhow::anyhow!("no MusicBrainz results"))?;

    let edits = mb_edits_for_missing_fields(&tags, candidate);
    if edits.is_empty() {
        return Ok(0);
    }
    let count = edits.len();
    write_id3v24_edits(std::path::Path::new(path), &edits)?;
    Ok(count)
}

fn match_candidate_to_track<'a>(
    candidates: &'a [MusicBrainzCandidate],
    track: &TrackRow,
) -> Option<&'a MusicBrainzCandidate> {
    // Try exact track number match first.
    if let Some(track_num) = track.track_number {
        if let Some(c) = candidates
            .iter()
            .find(|c| c.track_position == Some(track_num as i32))
        {
            return Some(c);
        }
    }
    // Fall back to title similarity.
    let track_title = track.track_title.as_deref()?;
    let normalized_title = track_title.to_lowercase();
    candidates.iter().max_by_key(|c| {
        let ct = c
            .track_title
            .as_deref()
            .or(Some(&c.title))
            .unwrap_or("")
            .to_lowercase();
        if ct == normalized_title {
            return 1000;
        }
        // Simple word overlap score.
        let title_words: Vec<&str> = normalized_title.split_whitespace().collect();
        let cand_words: Vec<&str> = ct.split_whitespace().collect();
        title_words
            .iter()
            .filter(|w| cand_words.contains(w))
            .count()
            * 100
            / title_words.len().max(1)
    })
}

fn apply_candidate_to_track(
    track: &TrackRow,
    candidate: &MusicBrainzCandidate,
) -> anyhow::Result<usize> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("no local file"))?;
    let tags = read_audio_tags(std::path::Path::new(path))?;
    let edits = mb_edits_for_missing_fields(&tags, candidate);
    if edits.is_empty() {
        return Ok(0);
    }
    let count = edits.len();
    write_id3v24_edits(std::path::Path::new(path), &edits)?;
    Ok(count)
}

async fn musicbrainz_feed_per_track(
    this: gpui::WeakEntity<LibraryApp>,
    cx: &mut gpui::AsyncApp,
    conn: &Arc<Mutex<Connection>>,
    downloadable: &[TrackRow],
    _feed_id: i64,
    total_count: usize,
) {
    let mut total_edits = 0usize;
    let mut processed = 0usize;
    for track in downloadable {
        let track_id = track.id;
        let progress = processed + 1;
        this.update(
            cx,
            move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                this.mb_status.insert(track_id, MbTrackStatus::Processing);
                this.status =
                    format!("MusicBrainz: processing track {progress}/{total_count} ...",);
                cx.notify();
            },
        )
        .ok();

        let conn2 = Arc::clone(conn);
        let track2 = track.clone();
        let result = cx
            .background_executor()
            .spawn(async move { musicbrainz_autotag_track(conn2, &track2) })
            .await;

        let status = match result {
            Ok(n) => {
                total_edits += n;
                MbTrackStatus::Done(n)
            }
            Err(err) => MbTrackStatus::Skipped(format!("{err:#}")),
        };
        processed += 1;

        let status_clone = status.clone();
        this.update(
            cx,
            move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                this.mb_status.insert(track_id, status_clone);
                cx.notify();
            },
        )
        .ok();

        if processed < total_count {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1100))
                .await;
        }
    }

    this.update(
        cx,
        move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
            this.status = format!(
                "MusicBrainz: {total_edits} edit{} across {} tracks",
                if total_edits == 1 { "" } else { "s" },
                processed,
            );
            this.reload();
            cx.notify();
        },
    )
    .ok();
}

fn build_tree(tracks: &[TrackRow]) -> LibraryTree {
    let mut artist_map: BTreeMap<String, BTreeMap<String, Vec<TrackRow>>> = BTreeMap::new();
    for track in tracks {
        let artist = track
            .album_artist_name
            .clone()
            .or_else(|| track.artist_name.clone())
            .or_else(|| track.feed_title.clone())
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let album = track
            .album_title
            .clone()
            .or_else(|| track.feed_title.clone())
            .unwrap_or_else(|| "Unknown Album".to_string());
        artist_map
            .entry(artist)
            .or_default()
            .entry(album)
            .or_default()
            .push(track.clone());
    }

    let artists = artist_map
        .into_iter()
        .map(|(artist_name, album_map)| {
            let albums = album_map
                .into_iter()
                .map(|(album_name, mut tracks)| {
                    tracks.sort_by(|a, b| a.track_number.cmp(&b.track_number));
                    let feed_id = tracks.first().map(|t| t.feed_id);
                    let image_href = tracks
                        .iter()
                        .find_map(|t| t.album_image_href.clone())
                        .or_else(|| tracks.iter().find_map(|t| t.track_image_href.clone()));
                    AlbumNode {
                        name: album_name,
                        feed_id,
                        image_href,
                        tracks,
                    }
                })
                .collect();
            ArtistNode {
                name: artist_name,
                albums,
            }
        })
        .collect();

    LibraryTree { artists }
}

fn cleanup_empty_parents(path: &std::path::Path) {
    let music_dir = config::config_path()
        .ok()
        .and_then(|p| config::load_config(&p).ok())
        .map(|c| c.music_dir);
    let mut dir = path.parent();
    while let Some(d) = dir {
        if music_dir.as_deref() == Some(d) {
            break;
        }
        if std::fs::read_dir(d)
            .map(|mut r| r.next().is_none())
            .unwrap_or(false)
        {
            let _ = std::fs::remove_dir(d);
            dir = d.parent();
        } else {
            break;
        }
    }
}

fn mb_edits_for_missing_fields(
    tags: &crate::audio_tags::AudioTags,
    candidate: &MusicBrainzCandidate,
) -> Vec<Id3v24Edit> {
    let mut edits = Vec::new();

    // Build TRCK as "pos/total" when both available.
    let trck_value = match (candidate.track_position, candidate.total_tracks) {
        (Some(pos), Some(total)) => Some(format!("{pos}/{total}")),
        _ => candidate.track_number.clone(),
    };

    // Standard text frames: (frame_label, existing_check, mb_value)
    let checks: Vec<(&str, bool, Option<String>)> = vec![
        ("TIT2", tags.title.is_some(), Some(candidate.title.clone())),
        ("TPE1", tags.artist.is_some(), candidate.artist.clone()),
        (
            "TALB",
            tags.album.is_some(),
            candidate.release_title.clone(),
        ),
        ("TRCK", tags.track_number.is_some(), trck_value),
        ("TDRC", tags.date.is_some(), candidate.release_date.clone()),
        (
            "TPUB",
            tag_has_frame(tags, "TPUB"),
            candidate.labels.first().cloned(),
        ),
        (
            "TSRC",
            tag_has_frame(tags, "TSRC"),
            candidate.isrcs.first().cloned(),
        ),
        (
            "TMED",
            tag_has_frame(tags, "TMED"),
            candidate.format.clone(),
        ),
        (
            "TPOS",
            tag_has_frame(tags, "TPOS"),
            candidate.medium_position.map(|p| p.to_string()),
        ),
        (
            "TSST",
            tag_has_frame(tags, "TSST"),
            candidate.medium_title.clone(),
        ),
        (
            "TLEN",
            tag_has_frame(tags, "TLEN"),
            candidate.track_length_ms.map(|ms| ms.to_string()),
        ),
        // TXXX frames
        (
            "TXXX:MusicBrainz Album Id",
            tags.custom.contains_key("MusicBrainz Album Id"),
            candidate.release_id.clone(),
        ),
        (
            "TXXX:MusicBrainz Release Group Id",
            tags.custom.contains_key("MusicBrainz Release Group Id"),
            candidate.release_group_id.clone(),
        ),
        (
            "TXXX:BARCODE",
            tags.custom.contains_key("BARCODE"),
            candidate.release_barcode.clone(),
        ),
        // UFID for MusicBrainz recording ID
        (
            "UFID:http://musicbrainz.org",
            tag_has_frame(tags, "UFID"),
            if candidate.recording_id.is_empty() {
                None
            } else {
                Some(candidate.recording_id.clone())
            },
        ),
    ];

    for (frame_label, has_existing, mb_value) in checks {
        if has_existing {
            continue;
        }
        if let Some(value) = mb_value {
            if !value.is_empty() {
                edits.push(Id3v24Edit {
                    frame_label: frame_label.to_string(),
                    value,
                });
            }
        }
    }
    edits
}

fn tag_has_frame(tags: &crate::audio_tags::AudioTags, frame_id: &str) -> bool {
    tags.fields.iter().any(|f| f.frame_id == frame_id)
}

fn track_row_to_api_track(track: &TrackRow) -> Track {
    Track {
        track_guid: Some(track.item_guid.clone()),
        feed_title: track.feed_title.clone(),
        title: track.track_title.clone(),
        duration_secs: track
            .duration_seconds
            .and_then(|seconds| seconds.try_into().ok()),
        track_number: track.track_number.and_then(|number| number.try_into().ok()),
        enclosure_url: track.enclosure_url.clone(),
        image_url: track.track_image_href.clone(),
        track_artist: track.artist_name.clone(),
        source_links: track.transcript_url.as_ref().map(|url| {
            vec![SourceEntityLink {
                entity_type: Some("track".into()),
                entity_id: Some(track.item_guid.clone()),
                link_type: Some("transcript".into()),
                url: Some(url.clone()),
                source: Some("rss".into()),
                extraction_path: Some("podcast:transcript@url".into()),
                ..Default::default()
            }]
        }),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for LibraryApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_text = self.status.clone();
        let status_color = if status_text.starts_with("Error:") {
            rgb(0xff6b6b)
        } else {
            muted()
        };

        let is_cached_tab = self.tab == LibraryTab::Cached;

        // Collect image URLs from tree, then fetch thumbnails (avoids borrow conflict).
        let urls: Vec<String> = {
            let tree = match self.tab {
                LibraryTab::Library => &self.tree,
                LibraryTab::Cached => &self.cached_tree,
            };
            tree.artists
                .iter()
                .flat_map(|a| &a.albums)
                .filter_map(|a| a.image_href.clone())
                .collect()
        };
        let mut album_thumbs: BTreeMap<String, Option<Arc<Image>>> = BTreeMap::new();
        for url in &urls {
            if !album_thumbs.contains_key(url.as_str()) {
                let img = self.thumbnail_for_url(Some(url), cx);
                album_thumbs.insert(url.clone(), img);
            }
        }

        let tree = match self.tab {
            LibraryTab::Library => &self.tree,
            LibraryTab::Cached => &self.cached_tree,
        };
        let left_items: Vec<AnyElement> = render_tree(
            tree,
            &self.expanded_artists,
            &self.expanded_albums,
            self.selected_id,
            is_cached_tab,
            &album_thumbs,
            cx,
        );

        let detail_pane = render_detail(
            &self.detail,
            self.busy_track,
            &self.mb_status,
            &album_thumbs,
            cx,
        );

        div()
            .size_full()
            .bg(bg())
            .text_color(text())
            .text_sm()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Tab bar
            .child(
                div()
                    .bg(surface())
                    .border_b_1()
                    .border_color(border())
                    .px(px(12.0))
                    .py(px(6.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(render_tab_button(
                        "Library",
                        LibraryTab::Library,
                        self.tab,
                        cx,
                    ))
                    .child(render_tab_button(
                        "Cached",
                        LibraryTab::Cached,
                        self.tab,
                        cx,
                    ))
                    .child(
                        div().flex_1().child(
                            div().text_right().child(
                                Button::new("lib-refresh")
                                    .label("Refresh")
                                    .ghost()
                                    .with_size(Size::XSmall)
                                    .text_color(rgb(0xffffff))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.reload();
                                        cx.notify();
                                    })),
                            ),
                        ),
                    ),
            )
            // Two panes
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    // Left pane: list
                    .child(
                        div()
                            .w(px(320.0))
                            .min_w(px(200.0))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .border_r_1()
                            .border_color(border())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(status_color)
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(border())
                                    .child(SharedString::from(status_text)),
                            )
                            .child(
                                div()
                                    .id("library-list")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .p(px(8.0))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.0))
                                            .children(left_items)
                                            .when(is_cached_tab && !self.list_is_empty(), |el| {
                                                el.child(
                                                    div().pt(px(8.0)).child(
                                                        Button::new("delete-all-cached")
                                                            .label("Delete All Cached")
                                                            .danger()
                                                            .with_size(Size::XSmall)
                                                            .text_color(rgb(0xffffff))
                                                            .on_click(cx.listener(
                                                                |this, _, _, cx| {
                                                                    this.delete_all_cached();
                                                                    cx.notify();
                                                                },
                                                            )),
                                                    ),
                                                )
                                            })
                                            .when(
                                                self.list_is_empty()
                                                    && !self.status.starts_with("Error:"),
                                                |el| {
                                                    el.child(
                                                        div()
                                                            .text_center()
                                                            .p(px(48.0))
                                                            .text_color(muted())
                                                            .child(div().mt(px(8.0)).child(
                                                                match self.tab {
                                                                    LibraryTab::Library => {
                                                                        "No library tracks yet"
                                                                    }
                                                                    LibraryTab::Cached => {
                                                                        "No cached files"
                                                                    }
                                                                },
                                                            )),
                                                    )
                                                },
                                            ),
                                    ),
                            ),
                    )
                    // Right pane: detail
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(detail_pane),
                    ),
            )
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

fn render_tab_button(
    label: &'static str,
    tab: LibraryTab,
    active: LibraryTab,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let is_active = tab == active;
    let mut btn = Button::new(SharedString::from(format!("lib-tab-{label}")))
        .label(label)
        .with_size(Size::Small);

    if is_active {
        btn = btn.primary();
    } else {
        btn = btn.ghost();
    }

    btn.text_color(rgb(0xffffff))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.tab = tab;
            this.reload();
            cx.notify();
        }))
        .into_any_element()
}

fn render_tree(
    tree: &LibraryTree,
    expanded_artists: &HashSet<String>,
    expanded_albums: &HashSet<(String, String)>,
    selected_id: Option<i64>,
    is_cached: bool,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> Vec<AnyElement> {
    let mut items = Vec::new();
    for artist in &tree.artists {
        let artist_expanded = expanded_artists.contains(&artist.name);
        let arrow = if artist_expanded {
            "\u{25BC}"
        } else {
            "\u{25B6}"
        };
        let album_count = artist.albums.len();
        let artist_name = artist.name.clone();

        items.push(
            div()
                .id(SharedString::from(format!("artist-{}", artist.name)))
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(|el| el.bg(rgb(0x1f2230)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_artist(&artist_name);
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(6.0))
                        .items_baseline()
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted())
                                .w(px(12.0))
                                .child(SharedString::from(arrow)),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(text())
                                .child(SharedString::from(artist.name.clone())),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted())
                                .child(SharedString::from(format!(
                                    "({album_count} album{})",
                                    if album_count == 1 { "" } else { "s" }
                                ))),
                        ),
                )
                .into_any_element(),
        );

        if artist_expanded {
            for album in &artist.albums {
                let album_key = (artist.name.clone(), album.name.clone());
                let album_expanded = expanded_albums.contains(&album_key);
                let arrow = if album_expanded {
                    "\u{25BC}"
                } else {
                    "\u{25B6}"
                };
                let track_count = album.tracks.len();
                let artist_for_toggle = artist.name.clone();
                let album_for_toggle = album.name.clone();
                let album_for_select = album.clone();
                let thumb_image = album
                    .image_href
                    .as_ref()
                    .and_then(|url| album_thumbs.get(url.as_str()))
                    .and_then(|opt| opt.clone());

                items.push(
                    div()
                        .id(SharedString::from(format!(
                            "album-{}-{}",
                            artist.name, album.name
                        )))
                        .pl(px(20.0))
                        .pr(px(8.0))
                        .py(px(3.0))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .hover(|el| el.bg(rgb(0x1f2230)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_album(&artist_for_toggle, &album_for_toggle);
                            this.select_album(&album_for_select);
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(6.0))
                                .items_center()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted())
                                        .w(px(12.0))
                                        .child(SharedString::from(arrow)),
                                )
                                .child(render_album_thumb(thumb_image.as_ref(), 24.0))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(accent())
                                        .child(SharedString::from(album.name.clone())),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted())
                                        .child(SharedString::from(format!("({track_count})",))),
                                ),
                        )
                        .into_any_element(),
                );

                if album_expanded {
                    for track in &album.tracks {
                        let track_clone_a = track.clone();
                        let track_clone_b = track.clone();
                        let is_selected = selected_id == Some(track.id);
                        let title = track
                            .track_title
                            .as_deref()
                            .unwrap_or("[untitled]")
                            .to_string();
                        let num = track
                            .track_number
                            .map(|n| format!("{n:02} - "))
                            .unwrap_or_default();

                        let mut row = div()
                            .id(SharedString::from(format!("tree-track-{}", track.id)))
                            .pl(px(44.0))
                            .pr(px(8.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .when(is_selected, |el| el.bg(rgb(0x252836)))
                            .hover(|el| el.bg(rgb(0x1f2230)));

                        if is_cached {
                            let path_for_delete = track.local_path.clone().unwrap_or_default();
                            row = row
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_track(&track_clone_a);
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_xs()
                                                .text_color(if is_selected {
                                                    accent()
                                                } else {
                                                    text()
                                                })
                                                .child(SharedString::from(format!("{num}{title}"))),
                                        )
                                        .child(
                                            Button::new(SharedString::from(format!(
                                                "del-tree-{}",
                                                track.id
                                            )))
                                            .label("Delete")
                                            .danger()
                                            .with_size(Size::XSmall)
                                            .text_color(rgb(0xffffff))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.delete_cached_file(path_for_delete.clone());
                                                cx.notify();
                                            })),
                                        ),
                                );
                        } else {
                            row = row
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_track(&track_clone_b);
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(if is_selected { accent() } else { text() })
                                        .child(SharedString::from(format!("{num}{title}"))),
                                );
                        }

                        items.push(row.into_any_element());
                    }
                }
            }
        }
    }
    items
}

fn render_detail(
    detail: &LibraryDetail,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    match detail {
        LibraryDetail::None => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_color(muted())
                    .text_center()
                    .child("Select an item to view details"),
            )
            .into_any_element(),

        LibraryDetail::Album(album) => {
            render_album_detail(album, busy_track, mb_status, album_thumbs, cx)
        }

        LibraryDetail::Track(track) => render_track_detail(track, busy_track, mb_status, cx),
    }
}

fn render_album_detail(
    album: &AlbumNode,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let album_clone = album.clone();
    let title = &album.name;
    let has_any_mb = mb_status
        .values()
        .any(|s| matches!(s, MbTrackStatus::Pending | MbTrackStatus::Processing));
    let feed_id = album.feed_id;
    let thumb_image = album
        .image_href
        .as_ref()
        .and_then(|url| album_thumbs.get(url.as_str()))
        .and_then(|opt| opt.clone());

    let track_rows: Vec<AnyElement> = album
        .tracks
        .iter()
        .map(|track| {
            let track_for_click = track.clone();
            let track_id = track.id;
            let in_library = track.is_in_library;
            let is_busy = busy_track == Some(track_id);
            let track_title = track
                .track_title
                .as_deref()
                .unwrap_or("[untitled]")
                .to_string();
            let num_str = track
                .track_number
                .map(|n| format!("{n}. "))
                .unwrap_or_default();
            let dur = track
                .duration_seconds
                .map(|s| format!("  ({}:{:02})", s / 60, s % 60))
                .unwrap_or_default();
            let mb = mb_status.get(&track_id);
            let mb_text = match mb {
                Some(MbTrackStatus::Pending) => Some("MB: pending"),
                Some(MbTrackStatus::Processing) => Some("MB: looking up..."),
                Some(MbTrackStatus::Done(0)) => Some("MB: no missing fields"),
                Some(MbTrackStatus::Done(_)) => Some("MB: done"),
                Some(MbTrackStatus::Skipped(_)) => Some("MB: skipped"),
                None => None,
            };

            div()
                .id(SharedString::from(format!("album-track-{track_id}")))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .hover(|el| el.bg(rgb(0x1f2230)))
                .child(
                    div()
                        .flex_1()
                        .child(SharedString::from(format!("{num_str}{track_title}{dur}")))
                        .when(mb_text.is_some(), |el| {
                            let color = match mb {
                                Some(MbTrackStatus::Done(n)) if *n > 0 => rgb(0x6bcc6b),
                                Some(MbTrackStatus::Skipped(_)) => rgb(0xff6b6b),
                                Some(MbTrackStatus::Processing) => rgb(0xffcc00),
                                _ => muted(),
                            };
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(color)
                                    .child(SharedString::from(mb_text.unwrap().to_string())),
                            )
                        }),
                )
                .child(
                    Button::new(SharedString::from(format!("lib-toggle-{track_id}")))
                        .label(if is_busy {
                            "Subscribing..."
                        } else if in_library {
                            "Unsubscribe"
                        } else {
                            "Subscribe"
                        })
                        .with_size(Size::XSmall)
                        .when(in_library, |btn| btn.primary())
                        .when(!in_library, |btn| btn.ghost())
                        .text_color(rgb(0xffffff))
                        .disabled(is_busy)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if in_library {
                                this.remove_track(track_id);
                            } else {
                                this.subscribe_track(track_for_click.clone(), cx);
                            }
                            cx.notify();
                        })),
                )
                .when(track.local_path.is_some(), |el| {
                    el.child(div().text_xs().text_color(rgb(0x6bcc6b)).child("dl'd"))
                })
                .into_any_element()
        })
        .collect();

    // Compute album metadata.
    let artist = album
        .tracks
        .iter()
        .find_map(|t| {
            t.album_artist_name
                .clone()
                .or_else(|| t.artist_name.clone())
        })
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let total_duration_secs: i64 = album.tracks.iter().filter_map(|t| t.duration_seconds).sum();
    let duration_str = if total_duration_secs > 0 {
        let mins = total_duration_secs / 60;
        let secs = total_duration_secs % 60;
        if mins >= 60 {
            format!("{}h {}m", mins / 60, mins % 60)
        } else {
            format!("{mins}:{secs:02}")
        }
    } else {
        String::new()
    };
    let downloaded = album
        .tracks
        .iter()
        .filter(|t| t.local_path.is_some())
        .count();

    // Buttons row.
    let mut buttons = div().flex().flex_row().gap(px(8.0));
    if let Some(fid) = feed_id {
        buttons = buttons.child(
            Button::new("unsub-btn")
                .label("Unsubscribe Feed")
                .danger()
                .with_size(Size::XSmall)
                .text_color(rgb(0xffffff))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.unsubscribe_feed(fid);
                    cx.notify();
                })),
        );
    }
    buttons = buttons.child(
        Button::new("mb-album-btn")
            .label("MusicBrainz")
            .ghost()
            .with_size(Size::XSmall)
            .text_color(rgb(0xffffff))
            .disabled(has_any_mb)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.musicbrainz_feed(album_clone.clone(), cx);
            })),
    );

    let track_count = album.tracks.len();
    let mut info_parts = vec![format!(
        "{track_count} track{}",
        if track_count == 1 { "" } else { "s" }
    )];
    if !duration_str.is_empty() {
        info_parts.push(duration_str);
    }
    if downloaded > 0 && downloaded < track_count {
        info_parts.push(format!("{downloaded} downloaded"));
    }

    div()
        .id("album-detail-scroll")
        .size_full()
        .overflow_y_scroll()
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        // Album art + title + artist header.
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(16.0))
                .child(render_album_thumb(thumb_image.as_ref(), 96.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::BOLD)
                                .text_color(accent())
                                .child(SharedString::from(title.clone())),
                        )
                        .child(div().text_color(text()).child(SharedString::from(artist)))
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted())
                                .child(SharedString::from(info_parts.join(" \u{00B7} "))),
                        ),
                ),
        )
        .child(buttons)
        .child(div().flex().flex_col().gap(px(2.0)).children(track_rows))
        .into_any_element()
}

fn render_track_detail(
    track: &TrackRow,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let track_id = track.id;
    let in_library = track.is_in_library;
    let is_busy = busy_track == Some(track_id);
    let track_for_click = track.clone();
    let track_for_mb = track.clone();
    let title = track.track_title.as_deref().unwrap_or("[untitled]");
    let has_file = track.local_path.is_some();
    let mb = mb_status.get(&track_id);
    let mb_busy = matches!(mb, Some(MbTrackStatus::Processing));

    let mb_label: SharedString = match mb {
        Some(MbTrackStatus::Processing) => "MusicBrainz...".into(),
        Some(MbTrackStatus::Done(n)) => SharedString::from(format!("MusicBrainz: {n} edits")),
        Some(MbTrackStatus::Skipped(reason)) => SharedString::from(format!("MB: {reason}")),
        _ => "MusicBrainz".into(),
    };

    // Build metadata rows — prefer ID3 tags from local file when available.
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut embedded_art: Option<Arc<Image>> = None;

    if let Some(path) = &track.local_path {
        if let Ok(tags) = read_audio_tags(std::path::Path::new(path)) {
            // Primary fields.
            if let Some(v) = &tags.title {
                rows.push(("Title".into(), v.clone()));
            }
            if let Some(v) = &tags.artist {
                rows.push(("Artist".into(), v.clone()));
            }
            if let Some(v) = &tags.album {
                rows.push(("Album".into(), v.clone()));
            }
            if let Some(v) = &tags.track_number {
                let s = match &tags.total_tracks {
                    Some(t) => format!("{v}/{t}"),
                    None => v.clone(),
                };
                rows.push(("Track #".into(), s));
            }
            if let Some(v) = &tags.date {
                rows.push(("Date".into(), v.clone()));
            }
            // Custom TXXX and other frames.
            for (key, value) in &tags.custom {
                rows.push((key.clone(), value.clone()));
            }
            // Raw ID3 fields not already covered.
            for field in &tags.fields {
                let dominated = matches!(
                    field.frame_id.as_str(),
                    "TIT2" | "TPE1" | "TALB" | "TRCK" | "TDRC"
                ) || tags.custom.contains_key(&field.frame_id);
                if !dominated {
                    rows.push((field.frame_id.clone(), field.value.clone()));
                }
            }
            // Embedded artwork.
            if let Some(art) = &tags.artwork {
                if !art.data.is_empty() {
                    let fmt =
                        ImageFormat::from_mime_type(&art.mime_type).unwrap_or(ImageFormat::Jpeg);
                    embedded_art = Some(Arc::new(Image::from_bytes(fmt, art.data.clone())));
                }
            }
        }
    }

    // If no tags were read, fall back to DB fields.
    if rows.is_empty() {
        let db_rows: Vec<(&str, Option<String>)> = vec![
            ("Title", track.track_title.clone()),
            ("Artist", track.artist_name.clone()),
            (
                "Album",
                track
                    .album_title
                    .clone()
                    .or_else(|| track.feed_title.clone()),
            ),
            ("Album Artist", track.album_artist_name.clone()),
            ("Track #", track.track_number.map(|n| n.to_string())),
            ("Disc #", track.disc_number.map(|n| n.to_string())),
            (
                "Duration",
                track
                    .duration_seconds
                    .map(|s| format!("{}:{:02}", s / 60, s % 60)),
            ),
        ];
        for (k, v) in db_rows {
            if let Some(val) = v {
                rows.push((k.into(), val));
            }
        }
    }

    // Always add feed + local file info at the bottom.
    if let Some(v) = &track.feed_title {
        rows.push(("Feed".into(), v.clone()));
    }
    if let Some(v) = &track.local_path {
        rows.push(("Local file".into(), v.clone()));
    }

    let mut detail = div()
        .id("track-detail-scroll")
        .size_full()
        .overflow_y_scroll()
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0));

    // Artwork + title header.
    if embedded_art.is_some() {
        detail = detail.child(
            div()
                .flex()
                .flex_row()
                .gap(px(12.0))
                .child(render_album_thumb(embedded_art.as_ref(), 80.0))
                .child(
                    div().flex().flex_col().gap(px(4.0)).child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .text_color(accent())
                            .child(SharedString::from(title.to_string())),
                    ),
                ),
        );
    } else {
        detail = detail.child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(accent())
                .child(SharedString::from(title.to_string())),
        );
    }

    detail = detail.child(
        div()
            .flex()
            .flex_row()
            .gap(px(8.0))
            .child(
                Button::new("lib-remove-btn")
                    .label(if is_busy {
                        "Subscribing..."
                    } else if in_library {
                        "Unsubscribe"
                    } else {
                        "Subscribe"
                    })
                    .with_size(Size::Small)
                    .when(in_library, |btn| btn.danger())
                    .when(!in_library, |btn| btn.primary())
                    .text_color(rgb(0xffffff))
                    .disabled(is_busy)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if in_library {
                            this.remove_track(track_id);
                        } else {
                            this.subscribe_track(track_for_click.clone(), cx);
                        }
                        cx.notify();
                    })),
            )
            .when(has_file, |el| {
                el.child(
                    Button::new("mb-track-btn")
                        .label(mb_label.clone())
                        .ghost()
                        .with_size(Size::Small)
                        .text_color(rgb(0xffffff))
                        .disabled(mb_busy)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.musicbrainz_track(track_for_mb.clone(), cx);
                        })),
                )
            }),
    );

    detail = detail.children(rows.into_iter().map(|(key, value)| {
        div()
            .flex()
            .flex_row()
            .gap(px(8.0))
            .child(
                div()
                    .w(px(120.0))
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(muted())
                    .child(SharedString::from(key)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(text())
                    .child(SharedString::from(value)),
            )
            .into_any_element()
    }));

    detail.into_any_element()
}

fn load_thumbnail_image(url: &str) -> anyhow::Result<Option<Arc<Image>>> {
    let cache_dir = thumbnail_cache_dir()?;
    std::fs::create_dir_all(&cache_dir)?;
    let key = thumbnail_cache_key(url);
    let image_path = cache_dir.join(format!("{key}.image"));
    let mime_path = cache_dir.join(format!("{key}.mime"));

    if image_path.exists() && mime_path.exists() {
        let bytes = std::fs::read(&image_path)?;
        let mime_type = std::fs::read_to_string(&mime_path)?;
        if !bytes.is_empty() && mime_type.trim().starts_with("image/") {
            let format = ImageFormat::from_mime_type(mime_type.trim()).unwrap_or(ImageFormat::Jpeg);
            return Ok(Some(Arc::new(Image::from_bytes(format, bytes))));
        }
    }

    let response = ReqwestClient::new().get(url).send()?.error_for_status()?;
    let mime_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .map(str::trim)
        .filter(|v| v.starts_with("image/"))
        .map(str::to_string);
    let Some(mime_type) = mime_type else {
        return Ok(None);
    };
    let bytes = response.bytes()?.to_vec();
    if bytes.is_empty() {
        return Ok(None);
    }
    std::fs::write(&image_path, &bytes)?;
    std::fs::write(&mime_path, mime_type.as_bytes())?;
    let format = ImageFormat::from_mime_type(&mime_type).unwrap_or(ImageFormat::Jpeg);
    Ok(Some(Arc::new(Image::from_bytes(format, bytes))))
}

fn thumbnail_cache_dir() -> anyhow::Result<std::path::PathBuf> {
    let cfg_path = config::config_path()?;
    let parent = cfg_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("config path has no parent: {}", cfg_path.display()))?;
    Ok(parent.join("thumbnail-cache"))
}

fn thumbnail_cache_key(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn render_album_thumb(image: Option<&Arc<Image>>, size: f32) -> AnyElement {
    if let Some(img_data) = image {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(px(4.0))
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                img(img_data.clone())
                    .w(px(size))
                    .h(px(size))
                    .object_fit(ObjectFit::Cover),
            )
            .into_any_element()
    } else {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(px(4.0))
            .bg(border())
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.0))
            .flex_shrink_0()
            .child("\u{1F3B5}")
            .into_any_element()
    }
}
