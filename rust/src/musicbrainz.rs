use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::blocking::Client as ReqwestClient;
use reqwest::Url;
use serde::Deserialize;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LookupMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<String>,
    pub total_tracks: Option<String>,
    pub duration_secs: Option<i64>,
    pub isrc: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicBrainzLookup {
    pub query: String,
    pub candidates: Vec<MusicBrainzCandidate>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MusicBrainzCandidate {
    pub recording_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub release_id: Option<String>,
    pub release_title: Option<String>,
    pub release_group_id: Option<String>,
    pub release_group_type: Option<String>,
    pub release_group_secondary_types: Vec<String>,
    pub release_date: Option<String>,
    pub country: Option<String>,
    pub release_status: Option<String>,
    pub release_packaging: Option<String>,
    pub release_barcode: Option<String>,
    pub release_disambiguation: Option<String>,
    pub format: Option<String>,
    pub medium_position: Option<i32>,
    pub medium_title: Option<String>,
    pub track_number: Option<String>,
    pub track_position: Option<i32>,
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
    pub track_disambiguation: Option<String>,
    pub track_length_ms: Option<i64>,
    pub total_tracks: Option<i32>,
    pub isrcs: Vec<String>,
    pub labels: Vec<String>,
    pub urls: Vec<String>,
    pub duration_ms: Option<i64>,
    pub musicbrainz_score: Option<i32>,
    pub similarity_score: i32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct RecordingSearchResponse {
    recordings: Vec<MbRecording>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbRecording {
    id: String,
    title: Option<String>,
    length: Option<i64>,
    score: Option<i32>,
    artist_credit: Vec<MbArtistCredit>,
    releases: Vec<MbRelease>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct MbArtistCredit {
    name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbRelease {
    id: String,
    title: Option<String>,
    date: Option<String>,
    country: Option<String>,
    status: Option<String>,
    packaging: Option<String>,
    barcode: Option<String>,
    disambiguation: Option<String>,
    score: Option<i32>,
    artist_credit: Vec<MbArtistCredit>,
    release_group: Option<MbReleaseGroup>,
    label_info: Vec<MbLabelInfo>,
    media: Vec<MbMedium>,
    relations: Vec<MbRelation>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbReleaseGroup {
    id: Option<String>,
    primary_type: Option<String>,
    secondary_types: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbMedium {
    position: Option<i32>,
    title: Option<String>,
    format: Option<String>,
    track_count: Option<i32>,
    tracks: Vec<MbTrack>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbTrack {
    number: Option<String>,
    position: Option<i32>,
    title: Option<String>,
    length: Option<i64>,
    artist_credit: Vec<MbArtistCredit>,
    recording: Option<MbTrackRecording>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbTrackRecording {
    id: Option<String>,
    title: Option<String>,
    length: Option<i64>,
    disambiguation: Option<String>,
    isrcs: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbLabelInfo {
    catalog_number: Option<String>,
    label: Option<MbLabel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct MbLabel {
    id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbRelation {
    #[serde(rename = "type")]
    relation_type: Option<String>,
    target_type: Option<String>,
    direction: Option<String>,
    url: Option<MbRelationUrl>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct MbRelationUrl {
    id: Option<String>,
    resource: Option<String>,
}

pub fn lookup_recordings(
    client: &ReqwestClient,
    metadata: &LookupMetadata,
    limit: i32,
) -> Result<MusicBrainzLookup> {
    let query = build_recording_query(metadata)?;
    let requested_limit = limit.max(0) as usize;
    let limit = limit.to_string();
    let mut url = Url::parse("https://musicbrainz.org/ws/2/recording")?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("query", &query);
        pairs.append_pair("limit", &limit);
        pairs.append_pair("fmt", "json");
        pairs.append_pair("inc", "artist-credits+releases+release-groups+media");
    }

    let response = client
        .get(url)
        .send()?
        .error_for_status()?
        .json::<RecordingSearchResponse>()?;

    let mut candidates = response
        .recordings
        .iter()
        .flat_map(|recording| candidates_from_recording(recording, metadata))
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.similarity_score.cmp(&a.similarity_score));
    candidates.truncate(requested_limit);
    enrich_candidates_with_release_details(client, &mut candidates);

    Ok(MusicBrainzLookup { query, candidates })
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ReleaseSearchResponse {
    releases: Vec<MbRelease>,
}

pub fn lookup_releases(
    client: &ReqwestClient,
    metadata: &LookupMetadata,
    limit: i32,
) -> Result<Vec<MusicBrainzCandidate>> {
    let query = build_release_query(metadata)?;
    let requested_limit = limit.max(0) as usize;
    let limit = limit.to_string();
    let mut url = Url::parse("https://musicbrainz.org/ws/2/release")?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("query", &query);
        pairs.append_pair("limit", &limit);
        pairs.append_pair("fmt", "json");
    }

    let response = client
        .get(url)
        .send()?
        .error_for_status()?
        .json::<ReleaseSearchResponse>()?;

    // For each release stub, fetch full details and build per-track candidates.
    let mut candidates = Vec::new();
    let mut fetched = 0;
    for release_stub in response.releases.iter().take(requested_limit) {
        if fetched > 0 {
            sleep(Duration::from_millis(1100));
        }
        let Ok(release) = fetch_release_detail(client, &release_stub.id) else {
            fetched += 1;
            continue;
        };
        let artist = artist_credit_name(&release.artist_credit)
            .or_else(|| artist_credit_name(&release_stub.artist_credit));
        for medium in &release.media {
            for track in &medium.tracks {
                let tc = track_context_from_medium_track(medium, track);
                let recording_id = track
                    .recording
                    .as_ref()
                    .and_then(|r| r.id.clone())
                    .unwrap_or_default();
                let title = tc
                    .track_title
                    .clone()
                    .or_else(|| track.title.clone())
                    .unwrap_or_default();
                let candidate = MusicBrainzCandidate {
                    recording_id,
                    title,
                    artist: tc.track_artist.clone().or_else(|| artist.clone()),
                    release_id: Some(release.id.clone()),
                    release_title: release.title.clone(),
                    release_group_id: release.release_group.as_ref().and_then(|g| g.id.clone()),
                    release_group_type: release
                        .release_group
                        .as_ref()
                        .and_then(|g| g.primary_type.clone()),
                    release_group_secondary_types: release
                        .release_group
                        .as_ref()
                        .map_or_else(Vec::new, |g| g.secondary_types.clone()),
                    release_date: release.date.clone(),
                    country: release.country.clone(),
                    release_status: release.status.clone(),
                    release_packaging: release.packaging.clone(),
                    release_barcode: release.barcode.clone(),
                    release_disambiguation: release.disambiguation.clone(),
                    format: tc.format.clone(),
                    medium_position: tc.medium_position,
                    medium_title: tc.medium_title.clone(),
                    track_number: tc.number.clone(),
                    track_position: tc.track_position,
                    track_title: tc.track_title.clone(),
                    track_artist: tc.track_artist.clone(),
                    track_disambiguation: tc.track_disambiguation.clone(),
                    track_length_ms: tc.track_length_ms,
                    total_tracks: tc.total_tracks,
                    isrcs: tc.isrcs.clone(),
                    labels: release_label_values(&release),
                    urls: release_url_values(&release),
                    duration_ms: tc.track_length_ms,
                    musicbrainz_score: release_stub.score,
                    similarity_score: 0,
                };
                candidates.push(candidate);
            }
        }
        fetched += 1;
    }

    Ok(candidates)
}

fn build_release_query(metadata: &LookupMetadata) -> Result<String> {
    let mut parts = Vec::new();
    push_lucene_part(&mut parts, "release", metadata.album.as_deref());
    push_lucene_part(&mut parts, "artist", metadata.artist.as_deref());
    push_lucene_part(&mut parts, "tracks", metadata.total_tracks.as_deref());

    if parts.is_empty() {
        return Err(anyhow!(
            "not enough metadata for MusicBrainz release lookup"
        ));
    }

    Ok(parts.join(" AND "))
}

fn enrich_candidates_with_release_details(
    client: &ReqwestClient,
    candidates: &mut [MusicBrainzCandidate],
) {
    let mut releases = HashMap::<String, Option<MbRelease>>::new();

    for candidate in candidates {
        let Some(release_id) = candidate.release_id.clone() else {
            continue;
        };
        if !releases.contains_key(&release_id) {
            // Keep follow-up release lookups under MusicBrainz's public rate limit.
            sleep(Duration::from_millis(1100));
            releases.insert(
                release_id.clone(),
                fetch_release_detail(client, &release_id).ok(),
            );
        }
        if let Some(Some(release)) = releases.get(&release_id) {
            merge_release_detail(candidate, release);
        }
    }
}

fn fetch_release_detail(client: &ReqwestClient, release_id: &str) -> Result<MbRelease> {
    let mut url = Url::parse(&format!(
        "https://musicbrainz.org/ws/2/release/{release_id}"
    ))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("fmt", "json");
        pairs.append_pair(
            "inc",
            "artist-credits+labels+recordings+release-groups+media+isrcs+url-rels",
        );
    }

    Ok(client.get(url).send()?.error_for_status()?.json()?)
}

fn merge_release_detail(candidate: &mut MusicBrainzCandidate, release: &MbRelease) {
    candidate.release_title = release.title.clone().or(candidate.release_title.clone());
    candidate.release_date = release.date.clone().or(candidate.release_date.clone());
    candidate.country = release.country.clone().or(candidate.country.clone());
    candidate.release_status = release.status.clone();
    candidate.release_packaging = release.packaging.clone();
    candidate.release_barcode = release.barcode.clone();
    candidate.release_disambiguation = release.disambiguation.clone();
    candidate.release_group_id = release
        .release_group
        .as_ref()
        .and_then(|group| group.id.clone())
        .or(candidate.release_group_id.clone());
    candidate.release_group_type = release
        .release_group
        .as_ref()
        .and_then(|group| group.primary_type.clone());
    candidate.release_group_secondary_types = release
        .release_group
        .as_ref()
        .map_or_else(Vec::new, |group| group.secondary_types.clone());
    candidate.labels = release_label_values(release);
    candidate.urls = release_url_values(release);

    if let Some(track) =
        release_track_context(release, &candidate.recording_id, Some(&candidate.title))
    {
        apply_track_context(candidate, &track);
    }
}

fn candidates_from_recording(
    recording: &MbRecording,
    metadata: &LookupMetadata,
) -> Vec<MusicBrainzCandidate> {
    if recording.releases.is_empty() {
        return vec![candidate_from_recording(recording, None, None, metadata)];
    }

    recording
        .releases
        .iter()
        .map(|release| {
            let track = release_track_context(release, &recording.id, recording.title.as_deref());
            candidate_from_recording(recording, Some(release), track, metadata)
        })
        .collect()
}

fn candidate_from_recording(
    recording: &MbRecording,
    release: Option<&MbRelease>,
    track: Option<TrackContext>,
    metadata: &LookupMetadata,
) -> MusicBrainzCandidate {
    let artist = artist_credit_name(&recording.artist_credit);
    let release_title = release.and_then(|release| release.title.clone());
    let candidate = MusicBrainzCandidate {
        recording_id: recording.id.clone(),
        title: recording.title.clone().unwrap_or_default(),
        artist,
        release_id: release.map(|release| release.id.clone()),
        release_title,
        release_group_id: release
            .and_then(|release| release.release_group.as_ref())
            .and_then(|group| group.id.clone()),
        release_group_type: release
            .and_then(|release| release.release_group.as_ref())
            .and_then(|group| group.primary_type.clone()),
        release_group_secondary_types: release
            .and_then(|release| release.release_group.as_ref())
            .map_or_else(Vec::new, |group| group.secondary_types.clone()),
        release_date: release.and_then(|release| release.date.clone()),
        country: release.and_then(|release| release.country.clone()),
        release_status: release.and_then(|release| release.status.clone()),
        release_packaging: release.and_then(|release| release.packaging.clone()),
        release_barcode: release.and_then(|release| release.barcode.clone()),
        release_disambiguation: release.and_then(|release| release.disambiguation.clone()),
        format: track.as_ref().and_then(|track| track.format.clone()),
        medium_position: track.as_ref().and_then(|track| track.medium_position),
        medium_title: track.as_ref().and_then(|track| track.medium_title.clone()),
        track_number: track.as_ref().and_then(|track| track.number.clone()),
        track_position: track.as_ref().and_then(|track| track.track_position),
        track_title: track.as_ref().and_then(|track| track.track_title.clone()),
        track_artist: track.as_ref().and_then(|track| track.track_artist.clone()),
        track_disambiguation: track
            .as_ref()
            .and_then(|track| track.track_disambiguation.clone()),
        track_length_ms: track.as_ref().and_then(|track| track.track_length_ms),
        total_tracks: track.as_ref().and_then(|track| track.total_tracks),
        isrcs: track
            .as_ref()
            .map_or_else(Vec::new, |track| track.isrcs.clone()),
        labels: release.map_or_else(Vec::new, release_label_values),
        urls: release.map_or_else(Vec::new, release_url_values),
        duration_ms: recording.length,
        musicbrainz_score: recording.score,
        similarity_score: 0,
    };

    MusicBrainzCandidate {
        similarity_score: score_candidate(metadata, &candidate),
        ..candidate
    }
}

#[derive(Clone, Debug)]
struct TrackContext {
    number: Option<String>,
    total_tracks: Option<i32>,
    format: Option<String>,
    medium_position: Option<i32>,
    medium_title: Option<String>,
    track_position: Option<i32>,
    track_title: Option<String>,
    track_artist: Option<String>,
    track_disambiguation: Option<String>,
    track_length_ms: Option<i64>,
    isrcs: Vec<String>,
}

fn release_track_context(
    release: &MbRelease,
    recording_id: &str,
    recording_title: Option<&str>,
) -> Option<TrackContext> {
    for medium in &release.media {
        for track in &medium.tracks {
            let recording_matches = track
                .recording
                .as_ref()
                .and_then(|recording| recording.id.as_deref())
                == Some(recording_id);
            let title_matches =
                recording_title
                    .zip(track.title.as_deref())
                    .is_some_and(|(recording, track)| {
                        normalized_text(recording) == normalized_text(track)
                    });
            if recording_matches || title_matches {
                return Some(track_context_from_medium_track(medium, track));
            }
        }
    }
    release.media.first().map(|medium| TrackContext {
        number: None,
        total_tracks: medium.track_count,
        format: medium.format.clone(),
        medium_position: medium.position,
        medium_title: medium.title.clone(),
        track_position: None,
        track_title: None,
        track_artist: None,
        track_disambiguation: None,
        track_length_ms: None,
        isrcs: Vec::new(),
    })
}

fn track_context_from_medium_track(medium: &MbMedium, track: &MbTrack) -> TrackContext {
    TrackContext {
        number: track.number.clone(),
        total_tracks: medium.track_count,
        format: medium.format.clone(),
        medium_position: medium.position,
        medium_title: medium.title.clone(),
        track_position: track.position,
        track_title: track
            .recording
            .as_ref()
            .and_then(|recording| recording.title.clone())
            .or_else(|| track.title.clone()),
        track_artist: artist_credit_name(&track.artist_credit),
        track_disambiguation: track
            .recording
            .as_ref()
            .and_then(|recording| recording.disambiguation.clone()),
        track_length_ms: track
            .recording
            .as_ref()
            .and_then(|recording| recording.length)
            .or(track.length),
        isrcs: track
            .recording
            .as_ref()
            .map_or_else(Vec::new, |recording| recording.isrcs.clone()),
    }
}

fn apply_track_context(candidate: &mut MusicBrainzCandidate, track: &TrackContext) {
    candidate.format = track.format.clone().or(candidate.format.clone());
    candidate.medium_position = track.medium_position.or(candidate.medium_position);
    candidate.medium_title = track
        .medium_title
        .clone()
        .or(candidate.medium_title.clone());
    candidate.track_number = track.number.clone().or(candidate.track_number.clone());
    candidate.track_position = track.track_position.or(candidate.track_position);
    candidate.track_title = track.track_title.clone().or(candidate.track_title.clone());
    candidate.track_artist = track
        .track_artist
        .clone()
        .or(candidate.track_artist.clone());
    candidate.track_disambiguation = track
        .track_disambiguation
        .clone()
        .or(candidate.track_disambiguation.clone());
    candidate.track_length_ms = track.track_length_ms.or(candidate.track_length_ms);
    candidate.total_tracks = track.total_tracks.or(candidate.total_tracks);
    if !track.isrcs.is_empty() {
        candidate.isrcs = track.isrcs.clone();
    }
}

fn release_label_values(release: &MbRelease) -> Vec<String> {
    release
        .label_info
        .iter()
        .filter_map(|info| {
            let label = info
                .label
                .as_ref()
                .and_then(|label| label.name.clone())
                .or_else(|| info.label.as_ref().and_then(|label| label.id.clone()));
            match (
                label,
                info.catalog_number
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
            ) {
                (Some(label), Some(catalog)) => Some(format!("{label} ({catalog})")),
                (Some(label), None) => Some(label),
                (None, Some(catalog)) => Some(catalog),
                (None, None) => None,
            }
        })
        .collect()
}

fn release_url_values(release: &MbRelease) -> Vec<String> {
    release
        .relations
        .iter()
        .filter_map(|relation| {
            let resource = relation
                .url
                .as_ref()
                .and_then(|url| url.resource.clone().or_else(|| url.id.clone()))?;
            let relation_type = relation.relation_type.as_deref().unwrap_or("url");
            let target_type = relation.target_type.as_deref().unwrap_or("url");
            let direction = relation.direction.as_deref().unwrap_or("forward");
            Some(format!(
                "{relation_type} ({target_type}, {direction}): {resource}"
            ))
        })
        .collect()
}

fn artist_credit_name(credits: &[MbArtistCredit]) -> Option<String> {
    let value = credits
        .iter()
        .filter_map(|credit| credit.name.as_deref())
        .collect::<Vec<_>>()
        .join(" ");
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn score_candidate(metadata: &LookupMetadata, candidate: &MusicBrainzCandidate) -> i32 {
    let mut score = 0.0;
    let mut total = 0.0;

    add_score(
        &mut score,
        &mut total,
        13.0,
        text_similarity(metadata.title.as_deref(), Some(&candidate.title)),
    );
    add_score(
        &mut score,
        &mut total,
        10.0,
        duration_similarity(metadata.duration_secs, candidate.duration_ms),
    );
    add_score(
        &mut score,
        &mut total,
        5.0,
        text_similarity(
            metadata.album.as_deref(),
            candidate.release_title.as_deref(),
        ),
    );
    add_score(
        &mut score,
        &mut total,
        4.0,
        text_similarity(metadata.artist.as_deref(), candidate.artist.as_deref()),
    );
    add_score(
        &mut score,
        &mut total,
        4.0,
        text_similarity(
            metadata.track_number.as_deref(),
            candidate.track_number.as_deref(),
        ),
    );

    let base = if total == 0.0 {
        candidate.musicbrainz_score.unwrap_or_default()
    } else {
        ((score / total) * 100.0).round() as i32
    };
    let mut bonus = 0;
    if candidate.format.as_deref() == Some("Digital Media") {
        bonus += 3;
    }
    if candidate.country.as_deref() == Some("XW") {
        bonus += 2;
    }
    base + bonus
}

fn add_score(score: &mut f64, total: &mut f64, weight: f64, similarity: Option<f64>) {
    if let Some(similarity) = similarity {
        *score += weight * similarity;
        *total += weight;
    }
}

fn duration_similarity(source_secs: Option<i64>, candidate_ms: Option<i64>) -> Option<f64> {
    let source_ms = source_secs? * 1000;
    let candidate_ms = candidate_ms?;
    let diff = (source_ms - candidate_ms).abs().min(30_000);
    Some(1.0 - (diff as f64 / 30_000.0))
}

fn text_similarity(source: Option<&str>, candidate: Option<&str>) -> Option<f64> {
    let source = normalized_text(source?);
    let candidate = normalized_text(candidate?);
    if source.is_empty() || candidate.is_empty() {
        return None;
    }
    if source == candidate {
        return Some(1.0);
    }

    let source_words = source.split_whitespace().collect::<Vec<_>>();
    let candidate_words = candidate.split_whitespace().collect::<Vec<_>>();
    if source_words.is_empty() || candidate_words.is_empty() {
        return Some(edit_similarity(&source, &candidate));
    }

    let matched = source_words
        .iter()
        .filter(|word| {
            candidate_words
                .iter()
                .any(|candidate| edit_similarity(word, candidate) > 0.6)
        })
        .count();
    let total = source_words.len().max(candidate_words.len());
    Some((matched as f64 / total as f64).max(edit_similarity(&source, &candidate) * 0.6))
}

fn edit_similarity(source: &str, candidate: &str) -> f64 {
    let max_len = source.chars().count().max(candidate.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - (levenshtein(source, candidate) as f64 / max_len as f64)
}

fn levenshtein(source: &str, candidate: &str) -> usize {
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let mut costs = (0..=candidate_chars.len()).collect::<Vec<_>>();
    for (source_idx, source_char) in source.chars().enumerate() {
        let mut previous = costs[0];
        costs[0] = source_idx + 1;
        for (candidate_idx, candidate_char) in candidate_chars.iter().enumerate() {
            let current = costs[candidate_idx + 1];
            costs[candidate_idx + 1] = if source_char == *candidate_char {
                previous
            } else {
                1 + previous.min(costs[candidate_idx].min(current))
            };
            previous = current;
        }
    }
    costs[candidate_chars.len()]
}

fn normalized_text(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.trim().to_string()
}

fn build_recording_query(metadata: &LookupMetadata) -> Result<String> {
    let mut parts = Vec::new();
    push_lucene_part(&mut parts, "recording", metadata.title.as_deref());
    push_lucene_part(&mut parts, "artist", metadata.artist.as_deref());
    push_lucene_part(&mut parts, "release", metadata.album.as_deref());
    push_lucene_part(&mut parts, "tnum", metadata.track_number.as_deref());
    push_lucene_part(&mut parts, "tracks", metadata.total_tracks.as_deref());
    push_lucene_part(&mut parts, "isrc", metadata.isrc.as_deref());
    if let Some(duration_secs) = metadata.duration_secs {
        parts.push(format!("qdur:{}", duration_secs / 2));
    }

    if parts.is_empty() {
        return Err(anyhow!("not enough metadata for MusicBrainz lookup"));
    }

    Ok(parts.join(" AND "))
}

fn push_lucene_part(parts: &mut Vec<String>, field: &str, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    parts.push(format!("{field}:({})", escape_lucene_query(value)));
}

fn escape_lucene_query(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if matches!(
            ch,
            '+' | '-'
                | '&'
                | '|'
                | '!'
                | '('
                | ')'
                | '{'
                | '}'
                | '['
                | ']'
                | '^'
                | '"'
                | '~'
                | '*'
                | '?'
                | ':'
                | '\\'
                | '/'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        build_recording_query, merge_release_detail, text_similarity, LookupMetadata, MbRelease,
        MusicBrainzCandidate,
    };

    #[test]
    fn builds_picard_style_recording_query() {
        let query = build_recording_query(&LookupMetadata {
            title: Some("A/B Song".into()),
            artist: Some("Artist".into()),
            album: Some("Album".into()),
            track_number: Some("3".into()),
            total_tracks: Some("10".into()),
            duration_secs: Some(240),
            isrc: Some("US1234567890".into()),
        })
        .expect("query");

        assert_eq!(
            query,
            "recording:(A\\/B Song) AND artist:(Artist) AND release:(Album) AND tnum:(3) AND tracks:(10) AND isrc:(US1234567890) AND qdur:120"
        );
    }

    #[test]
    fn text_similarity_handles_near_matches() {
        let score = text_similarity(Some("The Long Road"), Some("Long Road")).expect("score");
        assert!(score > 0.6, "expected near match score, got {score}");
    }

    #[test]
    fn release_detail_enriches_candidate_fields() {
        let release: MbRelease = serde_json::from_str(
            r#"{
                "id": "release-id",
                "title": "Album Title",
                "date": "2018-01-02",
                "country": "US",
                "status": "Official",
                "packaging": "Digipak",
                "barcode": "1234567890",
                "disambiguation": "limited edition",
                "release-group": {
                    "id": "release-group-id",
                    "primary-type": "Album",
                    "secondary-types": ["Live"]
                },
                "label-info": [
                    {
                        "catalog-number": "CAT-001",
                        "label": {
                            "id": "label-id",
                            "name": "Example Label"
                        }
                    }
                ],
                "media": [
                    {
                        "position": 1,
                        "title": "Disc 1",
                        "format": "Digital Media",
                        "track-count": 10,
                        "tracks": [
                            {
                                "number": "4",
                                "position": 4,
                                "title": "Recording Title",
                                "length": 201000,
                                "artist-credit": [{"name": "Track Artist"}],
                                "recording": {
                                    "id": "recording-id",
                                    "title": "Recording Title",
                                    "length": 200000,
                                    "disambiguation": "single edit",
                                    "isrcs": ["USABC1234567"]
                                }
                            }
                        ]
                    }
                ],
                "relations": [
                    {
                        "type": "official homepage",
                        "target-type": "url",
                        "direction": "forward",
                        "url": {
                            "id": "url-id",
                            "resource": "https://example.invalid/album"
                        }
                    }
                ]
            }"#,
        )
        .expect("release detail");
        let mut candidate = MusicBrainzCandidate {
            recording_id: "recording-id".into(),
            title: "Recording Title".into(),
            release_id: Some("release-id".into()),
            ..MusicBrainzCandidate::default()
        };

        merge_release_detail(&mut candidate, &release);

        assert_eq!(candidate.release_title.as_deref(), Some("Album Title"));
        assert_eq!(
            candidate.release_group_id.as_deref(),
            Some("release-group-id")
        );
        assert_eq!(candidate.release_group_type.as_deref(), Some("Album"));
        assert_eq!(
            candidate.release_group_secondary_types,
            vec!["Live".to_string()]
        );
        assert_eq!(candidate.release_status.as_deref(), Some("Official"));
        assert_eq!(candidate.release_packaging.as_deref(), Some("Digipak"));
        assert_eq!(candidate.release_barcode.as_deref(), Some("1234567890"));
        assert_eq!(candidate.format.as_deref(), Some("Digital Media"));
        assert_eq!(candidate.medium_position, Some(1));
        assert_eq!(candidate.medium_title.as_deref(), Some("Disc 1"));
        assert_eq!(candidate.track_number.as_deref(), Some("4"));
        assert_eq!(candidate.track_position, Some(4));
        assert_eq!(candidate.track_artist.as_deref(), Some("Track Artist"));
        assert_eq!(candidate.track_length_ms, Some(200000));
        assert_eq!(candidate.isrcs, vec!["USABC1234567".to_string()]);
        assert_eq!(
            candidate.labels,
            vec!["Example Label (CAT-001)".to_string()]
        );
        assert_eq!(
            candidate.urls,
            vec!["official homepage (url, forward): https://example.invalid/album".to_string()]
        );
    }

    #[test]
    fn digital_media_xw_candidate_scores_higher() {
        use super::score_candidate;

        let metadata = LookupMetadata {
            title: Some("Test Song".into()),
            artist: Some("Test Artist".into()),
            album: None,
            track_number: None,
            total_tracks: None,
            duration_secs: Some(180),
            isrc: None,
        };

        let base_candidate = MusicBrainzCandidate {
            recording_id: "rec-123".into(),
            title: "Test Song".into(),
            artist: Some("Test Artist".into()),
            duration_ms: Some(180_000),
            format: Some("CD".into()),
            country: Some("US".into()),
            ..MusicBrainzCandidate::default()
        };

        let digital_xw_candidate = MusicBrainzCandidate {
            format: Some("Digital Media".into()),
            country: Some("XW".into()),
            ..base_candidate.clone()
        };

        let base_score = score_candidate(&metadata, &base_candidate);
        let digital_xw_score = score_candidate(&metadata, &digital_xw_candidate);

        assert!(
            digital_xw_score > base_score,
            "Digital Media/XW candidate should score higher: {} > {}",
            digital_xw_score,
            base_score
        );
        assert_eq!(
            digital_xw_score - base_score,
            5,
            "Score difference should be 5 (3 for Digital Media + 2 for XW)"
        );
    }
}
