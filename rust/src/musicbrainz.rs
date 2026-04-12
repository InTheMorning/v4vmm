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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicBrainzCandidate {
    pub recording_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub release_id: Option<String>,
    pub release_title: Option<String>,
    pub release_group_id: Option<String>,
    pub release_date: Option<String>,
    pub country: Option<String>,
    pub format: Option<String>,
    pub track_number: Option<String>,
    pub total_tracks: Option<i32>,
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
    release_group: Option<MbReleaseGroup>,
    media: Vec<MbMedium>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct MbReleaseGroup {
    id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct MbMedium {
    format: Option<String>,
    track_count: Option<i32>,
    tracks: Vec<MbTrack>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct MbTrack {
    number: Option<String>,
    title: Option<String>,
    recording: Option<MbTrackRecording>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct MbTrackRecording {
    id: Option<String>,
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

    Ok(MusicBrainzLookup { query, candidates })
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
        release_date: release.and_then(|release| release.date.clone()),
        country: release.and_then(|release| release.country.clone()),
        format: track.as_ref().and_then(|track| track.format.clone()),
        track_number: track.as_ref().and_then(|track| track.number.clone()),
        total_tracks: track.as_ref().and_then(|track| track.total_tracks),
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
                return Some(TrackContext {
                    number: track.number.clone(),
                    total_tracks: medium.track_count,
                    format: medium.format.clone(),
                });
            }
        }
    }
    release.media.first().map(|medium| TrackContext {
        number: None,
        total_tracks: medium.track_count,
        format: medium.format.clone(),
    })
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

    if total == 0.0 {
        candidate.musicbrainz_score.unwrap_or_default()
    } else {
        ((score / total) * 100.0).round() as i32
    }
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
    use super::{build_recording_query, text_similarity, LookupMetadata};

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
}
