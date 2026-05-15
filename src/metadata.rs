use std::collections::{BTreeMap, BTreeSet};

use crate::api::*;
use crate::audio_tags::{id3v24_edit_label_is_writable, Id3Field, Id3v24Edit};
use crate::musicbrainz::{MusicBrainzCandidate, MusicBrainzLookup};
use crate::track_compare::{compare_track_tags, ComparisonRow, ComparisonStatus};

// Constants

pub const ID3V24_FRAME_IDS: &[&str] = &[
    "AENC", "APIC", "ASPI", "COMM", "COMR", "ENCR", "EQU2", "ETCO", "GEOB", "GRID", "LINK", "MCDI",
    "MLLT", "OWNE", "PRIV", "PCNT", "POPM", "POSS", "RBUF", "RVA2", "RVRB", "SEEK", "SIGN", "SYLT",
    "SYTC", "TALB", "TBPM", "TCOM", "TCON", "TCOP", "TDEN", "TDLY", "TDOR", "TDRC", "TDRL", "TDTG",
    "TENC", "TEXT", "TFLT", "TIPL", "TIT1", "TIT2", "TIT3", "TKEY", "TLAN", "TLEN", "TMCL", "TMED",
    "TMOO", "TOAL", "TOFN", "TOLY", "TOPE", "TOWN", "TPE1", "TPE2", "TPE3", "TPE4", "TPOS", "TPRO",
    "TPUB", "TRCK", "TRSN", "TRSO", "TSOA", "TSOP", "TSOT", "TSRC", "TSSE", "TSST", "TXXX", "UFID",
    "USER", "USLT", "WCOM", "WCOP", "WOAF", "WOAR", "WOAS", "WORS", "WPAY", "WPUB", "WXXX",
];

pub const ID3V24_FRAME_GROUPS: &[(&str, &str)] = &[
    (
        "identification-release-structure",
        "Identification / release structure",
    ),
    ("people-credits", "People / credits"),
    (
        "descriptive-technical-rights-text",
        "Descriptive / technical / rights text",
    ),
    ("url-link-frames", "URL link frames"),
    (
        "lyrics-comments-artwork-user-facing-content",
        "Lyrics / comments / artwork / user-facing content",
    ),
    (
        "identity-linking-private-registration",
        "Identity / linking / private / registration",
    ),
    (
        "timing-seeking-audio-analysis-playback-control",
        "Timing / seeking / audio-analysis / playback-control",
    ),
    (
        "music-disc-acquisition-commerce",
        "Music-disc / acquisition / commerce",
    ),
];

// Types

#[derive(Clone, Debug)]
pub struct ImageBytes {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[derive(Clone, Debug)]
pub struct TrackContext {
    pub track: Track,
    pub feed: Option<Feed>,
}

#[derive(Clone, Debug)]
pub struct TagCompareResult {
    pub path: String,
    pub rows: Vec<ComparisonRow>,
    pub file_image: Option<ImageBytes>,
    pub contributors: Vec<Contributor>,
    pub value_routes: Vec<PaymentRoute>,
    pub id3_fields: Vec<Id3Field>,
    pub total_tracks: Option<String>,
    pub format: Option<crate::audio_format::AudioFormat>,
}

#[derive(Clone, Debug)]
pub struct MusicBrainzLookupResult {
    pub lookup: MusicBrainzLookup,
    pub image: Option<ImageBytes>,
}

#[derive(Clone, Debug)]
pub struct AlignedCompareRow {
    pub row_id: String,
    pub field: String,
    pub rss_value: Option<String>,
    pub id3_value: Option<String>,
    pub id3_frame: Option<String>,
    pub musicbrainz_value: Option<String>,
    pub musicbrainz_key: Option<String>,
    pub id3_status: ComparisonStatus,
    pub musicbrainz_status: ComparisonStatus,
}

#[derive(Clone, Debug)]
pub struct PendingId3Edit {
    pub field: String,
    pub frame: String,
    pub value: String,
    pub source: MetadataColumn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataColumn {
    Rss,
    MusicBrainz,
}

#[derive(Clone, Debug)]
pub struct MetadataDragValue {
    pub row_id: String,
    pub field: String,
    pub frame: String,
    pub target_existing_value: Option<String>,
    pub value: String,
    pub source: MetadataColumn,
}

#[derive(Clone, Debug)]
pub struct MetadataGroupRow {
    pub key: Option<String>,
    pub label: String,
    pub expanded: bool,
    pub unused_count: usize,
}

#[derive(Clone, Debug)]
pub enum MetadataGridRow {
    Group(MetadataGroupRow),
    Data(AlignedCompareRow),
}

#[derive(Clone, Debug)]
pub struct WoarMetadataUrl {
    pub rss_value: Option<String>,
    pub rss_compare_value: Option<String>,
    pub id3_value: Option<String>,
    pub id3_compare_value: Option<String>,
    pub musicbrainz_value: Option<String>,
    pub musicbrainz_key: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id3FrameVersion {
    V22,
    V23Only,
    V24Only,
    V23V24,
    Unknown,
}

// Track/Feed helpers

pub fn compare_track_rows(
    track: &Track,
    feed: Option<&Feed>,
    tags: &crate::audio_tags::AudioTags,
) -> Vec<ComparisonRow> {
    let mut rows = compare_track_tags(track, tags);
    sanitize_comparison_rows(&mut rows);

    push_compare_row(&mut rows, "RSS track guid", track.track_guid.clone(), None);
    push_compare_row(
        &mut rows,
        "RSS feed guid",
        track
            .feed_guid
            .clone()
            .or_else(|| feed.and_then(|feed| feed.feed_guid.clone())),
        None,
    );
    push_compare_row(&mut rows, "Nostr handle", track_nostr(track), None);
    push_compare_row(&mut rows, "Website", track_website(track), None);
    let release_date = track_release_pubdate(track).or_else(|| feed.and_then(feed_release_pubdate));
    push_compare_row(&mut rows, "Release date", release_date.clone(), None);
    push_compare_row(
        &mut rows,
        "Duration",
        track.duration_secs.map(fmt_dur),
        None,
    );
    push_compare_row(&mut rows, "Artwork", artwork_url(track, feed), None);
    push_compare_row(
        &mut rows,
        "Source format",
        track.enclosure_type.clone(),
        None,
    );

    if let Some(feed) = feed {
        push_if_differs(
            &mut rows,
            "RSS item pubdate",
            track_release_pubdate(track),
            release_date,
        );
        push_if_differs(
            &mut rows,
            "RSS feed nostr handle",
            feed_nostr(feed),
            track_nostr(track),
        );
        push_if_differs(
            &mut rows,
            "RSS feed website",
            feed_website(feed),
            track_website(track),
        );
    }

    rows
}

pub fn push_if_differs(
    rows: &mut Vec<ComparisonRow>,
    field: &'static str,
    feed_value: Option<String>,
    track_value: Option<String>,
) {
    if normalized_source_value(feed_value.as_deref())
        != normalized_source_value(track_value.as_deref())
    {
        push_compare_row(rows, field, feed_value, None);
    }
}

pub fn push_compare_row(
    rows: &mut Vec<ComparisonRow>,
    field: &'static str,
    source_value: Option<String>,
    tag_value: Option<String>,
) {
    let source_value = drop_placeholder_source_text(source_value);
    if normalized_compare_value(source_value.as_deref()).is_some()
        || normalized_compare_value(tag_value.as_deref()).is_some()
    {
        let status = compare_id3_field_values(field, source_value.as_deref(), tag_value.as_deref());
        rows.push(ComparisonRow {
            field,
            source_value,
            tag_value,
            status,
        });
    }
}

fn sanitize_comparison_rows(rows: &mut Vec<ComparisonRow>) {
    for row in rows.iter_mut() {
        row.source_value = drop_placeholder_source_text(row.source_value.take());
        row.status = compare_id3_field_values(
            row.field,
            row.source_value.as_deref(),
            row.tag_value.as_deref(),
        );
    }
    rows.retain(|row| {
        normalized_compare_value(row.source_value.as_deref()).is_some()
            || normalized_compare_value(row.tag_value.as_deref()).is_some()
    });
}

pub fn track_nostr(track: &Track) -> Option<String> {
    nostr_from_ids(track.source_ids.as_deref())
}

pub fn feed_nostr(feed: &Feed) -> Option<String> {
    nostr_from_ids(feed.source_ids.as_deref())
}

pub fn nostr_from_ids(ids: Option<&[SourceEntityId]>) -> Option<String> {
    ids?.iter().find_map(|id| {
        if id.scheme.as_deref() == Some("nostr_npub") {
            id.value.clone()
        } else {
            None
        }
    })
}

pub fn track_website(track: &Track) -> Option<String> {
    website_from_links(track.source_links.as_deref())
}

pub fn feed_website(feed: &Feed) -> Option<String> {
    website_from_links(feed.source_links.as_deref())
}

pub fn track_transcript_url(track: &Track) -> Option<String> {
    transcript_from_links(track.source_links.as_deref())
}

pub fn track_artwork_url(track_context: &TrackContext) -> Option<String> {
    artwork_url(&track_context.track, track_context.feed.as_ref())
}

pub fn artwork_url(track: &Track, feed: Option<&Feed>) -> Option<String> {
    track
        .image_url
        .clone()
        .or_else(|| feed.and_then(|feed| feed.image_url.clone()))
}

pub fn website_from_links(links: Option<&[SourceEntityLink]>) -> Option<String> {
    links?.iter().find_map(|link| {
        let link_type = link.link_type.as_deref()?;
        if link_type == "website" || link_type == "web_page" {
            link.url.clone()
        } else {
            None
        }
    })
}

pub fn transcript_from_links(links: Option<&[SourceEntityLink]>) -> Option<String> {
    links?.iter().find_map(|link| {
        let url = link.url.as_deref()?;
        let link_type = link
            .link_type
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let path = link
            .extraction_path
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if link_type.contains("transcript")
            || link_type.contains("caption")
            || link_type.contains("subtitle")
            || path.contains("transcript")
            || transcript_url_extension(url)
        {
            Some(url.to_string())
        } else {
            None
        }
    })
}

fn transcript_url_extension(url: &str) -> bool {
    let path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    [".srt", ".vtt", ".lrc", ".sub", ".sbv", ".ttml", ".dfxp"]
        .iter()
        .any(|extension| path.ends_with(extension))
}

pub fn track_release_pubdate(track: &Track) -> Option<String> {
    release_pubdate_from_claims(track.source_release_claims.as_deref())
        .or_else(|| track.pub_date.and_then(fmt_date))
}

pub fn feed_release_pubdate(feed: &Feed) -> Option<String> {
    release_pubdate_from_claims(feed.source_release_claims.as_deref())
        .or_else(|| feed.release_date.and_then(fmt_date))
        .or_else(|| feed.oldest_item_at.and_then(fmt_date))
        .or_else(|| {
            feed.tracks
                .as_deref()?
                .iter()
                .filter_map(|track| track.pub_date)
                .min()
                .and_then(fmt_date)
        })
}

pub fn musicindex_release_date(track_context: &TrackContext) -> Option<String> {
    track_release_pubdate(&track_context.track)
        .or_else(|| track_context.feed.as_ref().and_then(feed_release_pubdate))
}

pub fn release_pubdate_from_claims(claims: Option<&[SourceReleaseClaim]>) -> Option<String> {
    claims?.iter().find_map(|claim| {
        if claim.claim_type.as_deref() != Some("release_date") {
            return None;
        }

        let value = claim.claim_value.as_deref()?;
        value
            .parse::<i64>()
            .ok()
            .and_then(fmt_date)
            .or_else(|| Some(value.to_string()))
    })
}

pub fn normalized_compare_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn source_text_missing(value: Option<&str>) -> bool {
    value.is_none_or(source_text_is_placeholder)
}

pub(crate) fn drop_placeholder_source_text(value: Option<String>) -> Option<String> {
    value.filter(|value| !source_text_missing(Some(value.as_str())))
}

pub(crate) fn sanitize_track_context_source_text(context: &mut TrackContext) {
    sanitize_track_source_text(&mut context.track);
    if let Some(feed) = &mut context.feed {
        sanitize_feed_source_text(feed);
    }
}

pub(crate) fn sanitize_track_source_text(track: &mut Track) {
    track.feed_title = drop_placeholder_source_text(track.feed_title.take());
    track.feed_url = drop_placeholder_source_text(track.feed_url.take());
    track.title = drop_placeholder_source_text(track.title.take());
    track.name = drop_placeholder_source_text(track.name.take());
    track.description = drop_placeholder_source_text(track.description.take());
    track.enclosure_url = drop_placeholder_source_text(track.enclosure_url.take());
    track.enclosure_type = drop_placeholder_source_text(track.enclosure_type.take());
    track.image_url = drop_placeholder_source_text(track.image_url.take());
    track.track_artist = drop_placeholder_source_text(track.track_artist.take());
    track.release_artist = drop_placeholder_source_text(track.release_artist.take());
    track.publisher_text = drop_placeholder_source_text(track.publisher_text.take());
    sanitize_source_links(track.source_links.as_mut());
    sanitize_source_ids(track.source_ids.as_mut());
    sanitize_source_release_claims(track.source_release_claims.as_mut());
    sanitize_source_contributors(track.source_contributors.as_mut());
}

pub(crate) fn sanitize_feed_source_text(feed: &mut Feed) {
    feed.title = drop_placeholder_source_text(feed.title.take());
    feed.name = drop_placeholder_source_text(feed.name.take());
    feed.feed_url = drop_placeholder_source_text(feed.feed_url.take());
    feed.release_artist = drop_placeholder_source_text(feed.release_artist.take());
    feed.release_artist_sort = drop_placeholder_source_text(feed.release_artist_sort.take());
    feed.raw_medium = drop_placeholder_source_text(feed.raw_medium.take());
    feed.release_kind = drop_placeholder_source_text(feed.release_kind.take());
    feed.publisher_text = drop_placeholder_source_text(feed.publisher_text.take());
    feed.language = drop_placeholder_source_text(feed.language.take());
    feed.description = drop_placeholder_source_text(feed.description.take());
    feed.image_url = drop_placeholder_source_text(feed.image_url.take());
    sanitize_source_links(feed.source_links.as_mut());
    sanitize_source_ids(feed.source_ids.as_mut());
    sanitize_source_release_claims(feed.source_release_claims.as_mut());
    sanitize_source_contributors(feed.source_contributors.as_mut());
    if let Some(tracks) = &mut feed.tracks {
        for track in tracks {
            sanitize_track_source_text(track);
        }
    }
}

fn sanitize_source_links(links: Option<&mut Vec<SourceEntityLink>>) {
    let Some(links) = links else {
        return;
    };
    for link in links.iter_mut() {
        link.url = drop_placeholder_source_text(link.url.take());
    }
    links.retain(|link| link.url.is_some());
}

fn sanitize_source_ids(ids: Option<&mut Vec<SourceEntityId>>) {
    let Some(ids) = ids else {
        return;
    };
    for id in ids.iter_mut() {
        id.value = drop_placeholder_source_text(id.value.take());
    }
}

fn sanitize_source_release_claims(claims: Option<&mut Vec<SourceReleaseClaim>>) {
    let Some(claims) = claims else {
        return;
    };
    for claim in claims.iter_mut() {
        claim.claim_value = drop_placeholder_source_text(claim.claim_value.take());
    }
}

fn sanitize_source_contributors(contributors: Option<&mut Vec<Contributor>>) {
    let Some(contributors) = contributors else {
        return;
    };
    for contributor in contributors.iter_mut() {
        contributor.name = drop_placeholder_source_text(contributor.name.take());
        contributor.role = drop_placeholder_source_text(contributor.role.take());
        contributor.href = drop_placeholder_source_text(contributor.href.take());
        contributor.img = drop_placeholder_source_text(contributor.img.take());
        contributor.npub = drop_placeholder_source_text(contributor.npub.take());
        contributor.group_name = drop_placeholder_source_text(contributor.group_name.take());
    }
    contributors.retain(|contributor| contributor.name.is_some());
}

pub(crate) fn source_text_is_placeholder(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return true;
    }

    let scan = source_placeholder_scan(trimmed);
    scan.residual_text.is_empty()
        && (scan.placeholder_count >= 3 || scan.saw_ellipsis || scan.saw_markup_or_entity)
}

fn source_placeholder_char(ch: char) -> bool {
    matches!(ch, '.' | '\u{2026}')
}

#[derive(Debug, Default)]
struct SourcePlaceholderScan {
    residual_text: String,
    placeholder_count: usize,
    saw_ellipsis: bool,
    saw_markup_or_entity: bool,
}

fn source_placeholder_scan(value: &str) -> SourcePlaceholderScan {
    let mut rest = value;
    let mut scan = SourcePlaceholderScan::default();
    while !rest.is_empty() {
        if let Some(consumed) = placeholder_entity_len(rest) {
            scan.saw_markup_or_entity = true;
            if rest[..consumed].to_ascii_lowercase().contains("hellip")
                || rest[..consumed].contains("8230")
                || rest[..consumed].to_ascii_lowercase().contains("2026")
                || rest[..consumed].to_ascii_lowercase().contains("mldr")
            {
                scan.saw_ellipsis = true;
                scan.placeholder_count += 1;
            }
            rest = &rest[consumed..];
            continue;
        }

        let Some(ch) = rest.chars().next() else {
            break;
        };
        if ch.is_whitespace() || source_ignorable_format_char(ch) {
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if source_placeholder_char(ch) {
            scan.placeholder_count += 1;
            scan.saw_ellipsis |= ch == '\u{2026}';
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if let Some(consumed) = placeholder_markup_len(rest) {
            scan.saw_markup_or_entity = true;
            rest = &rest[consumed..];
            continue;
        }
        scan.residual_text.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    scan
}

fn source_ignorable_format_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{00A0}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}'
    )
}

fn placeholder_entity_len(value: &str) -> Option<usize> {
    const PLACEHOLDER_ENTITIES: &[&str] = &[
        "&hellip;", "&mldr;", "&#8230;", "&#x2026;", "&nbsp;", "&#160;", "&#xa0;", "&#x00a0;",
    ];
    let value = value.to_ascii_lowercase();
    PLACEHOLDER_ENTITIES
        .iter()
        .find(|entity| value.starts_with(**entity))
        .map(|entity| entity.len())
}

fn placeholder_markup_len(value: &str) -> Option<usize> {
    const PLACEHOLDER_MARKUP: &[&str] = &[
        "<p>", "</p>", "<br>", "<br/>", "<br />", "<div>", "</div>", "<span>", "</span>",
    ];
    let value = value.to_ascii_lowercase();
    PLACEHOLDER_MARKUP
        .iter()
        .find(|markup| value.starts_with(**markup))
        .map(|markup| markup.len())
}

fn normalized_source_value(value: Option<&str>) -> Option<String> {
    normalized_compare_value(value).filter(|value| !source_text_missing(Some(value.as_str())))
}

// MusicBrainz helpers

pub fn musicbrainz_release_summary(candidate: &MusicBrainzCandidate) -> String {
    let mut parts = Vec::new();
    if let Some(country) = &candidate.country {
        parts.push(country.clone());
    }
    if let Some(format) = &candidate.format {
        parts.push(format.clone());
    }
    if let Some(tracks) = candidate.total_tracks {
        parts.push(format!("{tracks} tracks"));
    }

    let mut value = if parts.is_empty() {
        candidate
            .release_title
            .clone()
            .unwrap_or_else(|| candidate.title.clone())
    } else {
        parts.join(" - ")
    };

    if let Some(date) = &candidate.release_date {
        value.push_str(&format!(" ({date})"));
    }
    value
}

pub fn musicbrainz_release_option_label(candidate: &MusicBrainzCandidate) -> String {
    let release = candidate
        .release_title
        .clone()
        .unwrap_or_else(|| candidate.title.clone());
    format!("{} - {}", musicbrainz_release_summary(candidate), release)
}

pub fn musicbrainz_subtitle(
    frame_musicbrainz_selected: usize,
    result: &MusicBrainzLookupResult,
    candidate: &MusicBrainzCandidate,
) -> String {
    let rank = if result
        .lookup
        .candidates
        .get(frame_musicbrainz_selected)
        .is_some()
    {
        frame_musicbrainz_selected + 1
    } else {
        1
    };
    let score = if let Some(musicbrainz_score) = candidate.musicbrainz_score {
        format!(
            "Best: #{} · {}% local · {} MB",
            rank, candidate.similarity_score, musicbrainz_score
        )
    } else {
        format!("Best: #{} · {}% local", rank, candidate.similarity_score)
    };
    if let Some(release_id) = &candidate.release_id {
        format!("{score} · {release_id}")
    } else {
        format!("{score} · {}", candidate.recording_id)
    }
}

// Metadata grid/rows

pub fn expand_woar_metadata_rows(rows: Vec<MetadataGridRow>) -> Vec<MetadataGridRow> {
    rows.into_iter()
        .flat_map(|row| match row {
            MetadataGridRow::Data(row)
                if row.id3_frame.as_deref().map(id3_frame_base) == Some("WOAR") =>
            {
                let urls = woar_metadata_urls(&row);
                if urls.is_empty() {
                    vec![MetadataGridRow::Data(row)]
                } else {
                    urls.into_iter()
                        .enumerate()
                        .map(|(index, url)| {
                            let label = if index == 0 {
                                row.field.clone()
                            } else {
                                format!("{} {}", row.field, index + 1)
                            };
                            MetadataGridRow::Data(AlignedCompareRow {
                                row_id: compare_row_id(&label),
                                field: label,
                                rss_value: url.rss_value,
                                id3_value: url.id3_value,
                                id3_frame: Some("WOAR".into()),
                                musicbrainz_value: url.musicbrainz_value,
                                musicbrainz_key: url.musicbrainz_key,
                                id3_status: compare_optional_values(
                                    url.rss_compare_value.as_deref(),
                                    url.id3_compare_value.as_deref(),
                                ),
                                musicbrainz_status: ComparisonStatus::MissingSource,
                            })
                        })
                        .collect()
                }
            }
            row => vec![row],
        })
        .collect()
}

pub fn woar_metadata_urls(row: &AlignedCompareRow) -> Vec<WoarMetadataUrl> {
    let mut seen = BTreeSet::new();
    let mut urls = Vec::new();
    add_woar_source_urls(
        &mut urls,
        &mut seen,
        row.rss_value.as_deref(),
        MetadataColumn::Rss,
        row.musicbrainz_key.clone(),
    );
    add_woar_source_urls(
        &mut urls,
        &mut seen,
        row.musicbrainz_value.as_deref(),
        MetadataColumn::MusicBrainz,
        row.musicbrainz_key.clone(),
    );
    add_woar_id3_urls(&mut urls, &mut seen, row.id3_value.as_deref());
    urls
}

pub fn add_woar_source_urls(
    urls: &mut Vec<WoarMetadataUrl>,
    seen: &mut BTreeSet<String>,
    value: Option<&str>,
    source: MetadataColumn,
    musicbrainz_key: Option<String>,
) {
    let Some(value) = value else {
        return;
    };
    for url in split_joined_metadata_values(value) {
        let key = url.to_ascii_lowercase();
        if seen.insert(key) {
            urls.push(WoarMetadataUrl {
                rss_value: (source == MetadataColumn::Rss).then(|| url.clone()),
                rss_compare_value: (source == MetadataColumn::Rss).then(|| url.clone()),
                id3_value: None,
                id3_compare_value: None,
                musicbrainz_value: (source == MetadataColumn::MusicBrainz).then(|| url.clone()),
                musicbrainz_key: (source == MetadataColumn::MusicBrainz)
                    .then(|| musicbrainz_key.clone())
                    .flatten(),
            });
        }
    }
}

pub fn add_woar_id3_urls(
    urls: &mut Vec<WoarMetadataUrl>,
    seen: &mut BTreeSet<String>,
    value: Option<&str>,
) {
    let Some(value) = value else {
        return;
    };
    for url in split_joined_metadata_values(value) {
        let compare_url = embedded_url(&url).unwrap_or_else(|| url.clone());
        let key = compare_url.to_ascii_lowercase();
        if let Some(existing) = urls.iter_mut().find(|entry| {
            entry
                .rss_compare_value
                .as_ref()
                .or(entry.musicbrainz_value.as_ref())
                .is_some_and(|source_url| source_url.eq_ignore_ascii_case(&compare_url))
        }) {
            existing.id3_value = Some(url.clone());
            existing.id3_compare_value = Some(compare_url);
        } else if seen.insert(key) {
            urls.push(WoarMetadataUrl {
                rss_value: None,
                rss_compare_value: None,
                id3_value: Some(url.clone()),
                id3_compare_value: Some(compare_url),
                musicbrainz_value: None,
                musicbrainz_key: None,
            });
        }
    }
}

pub fn embedded_url(value: &str) -> Option<String> {
    if let Some(start) = value.find("https://").or_else(|| value.find("http://")) {
        let url = value[start..]
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches([')', ',', ';'])
            .to_string();
        if !url.is_empty() {
            return Some(url);
        }
    }
    // RSS sometimes lists URLs without a scheme (e.g. "www.example.com"). The
    // WOAR auto-populator wraps those as `download for free (url, forward): www…`,
    // so on read-back we still want to recover the raw host so it matches the
    // RSS-supplied value when the wrapped frame is the only thing in the file.
    if let Some((_, rest)) = value.split_once(": ") {
        let candidate = rest
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches([')', ',', ';']);
        if candidate.starts_with("www.") {
            return Some(candidate.to_string());
        }
    }
    None
}

pub fn row_contains_multiple_urls(row: &AlignedCompareRow) -> bool {
    [
        row.rss_value.as_deref(),
        row.id3_value.as_deref(),
        row.musicbrainz_value.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| split_joined_metadata_values(value).len() > 1)
}

pub fn split_joined_metadata_values(value: &str) -> Vec<String> {
    value
        .split('·')
        .filter_map(|part| normalized_compare_value(Some(part)))
        .collect()
}

pub fn auto_populated_pending_id3_edits(
    rows: &[MetadataGridRow],
    explicit: &BTreeMap<String, PendingId3Edit>,
    suppressed: &BTreeSet<String>,
    format: Option<crate::audio_format::AudioFormat>,
) -> BTreeMap<String, PendingId3Edit> {
    use crate::audio_format::AudioFormat;
    let non_mp3 = matches!(format, Some(f) if f != AudioFormat::Mp3);
    let target_for = |frame: &str| -> String {
        if let Some(format) = format.filter(|f| *f != AudioFormat::Mp3) {
            if let Some(dest) = frame_destination_for_format(frame, format) {
                return dest;
            }
        }
        pending_id3_target_key(frame)
    };
    let frame_writable_in_format = |frame: &str| -> bool {
        match format {
            Some(format) if format != AudioFormat::Mp3 => {
                frame_destination_for_format(frame, format).is_some()
            }
            _ => true,
        }
    };

    // Pre-pass: which destinations does the file already populate? On non-MP3
    // formats sibling frames collapse onto the same destination (TYER → DATE,
    // shared with TDRC), so once one is filled we shouldn't auto-stage the
    // other from the same source.
    let mut populated_destinations: BTreeSet<String> = BTreeSet::new();
    if non_mp3 {
        for row in rows {
            if let MetadataGridRow::Data(row) = row {
                if normalized_compare_value(row.id3_value.as_deref()).is_some() {
                    if let Some(frame) = row.id3_frame.as_deref() {
                        populated_destinations.insert(target_for(frame));
                    }
                }
            }
        }
    }

    let mut pending = explicit.clone();
    for row in rows {
        let MetadataGridRow::Data(row) = row else {
            continue;
        };
        if pending.contains_key(&row.row_id)
            || suppressed.contains(&row.row_id)
            || normalized_compare_value(row.id3_value.as_deref()).is_some()
        {
            continue;
        }
        let Some(frame) = row.id3_frame.as_deref() else {
            continue;
        };
        if !id3v24_drag_copy_frame_is_writable(frame) {
            continue;
        }
        if !frame_writable_in_format(frame) {
            continue;
        }
        let dest = target_for(frame);
        if non_mp3 && populated_destinations.contains(&dest) {
            continue;
        }
        let Some((source, source_value)) = auto_id3_source_value(row) else {
            continue;
        };
        let existing_value = pending
            .values()
            .find(|edit| target_for(&edit.frame) == dest)
            .map(|edit| edit.value.as_str())
            .or(row.id3_value.as_deref());
        let Some(value) = format_source_value_for_id3v24(
            frame,
            &row.field,
            source,
            existing_value,
            &source_value,
        ) else {
            continue;
        };
        // Aliased frames (TYER + TDRC → DATE on Vorbis) would otherwise both
        // stage the same value; keep only the first.
        if non_mp3
            && pending
                .values()
                .any(|edit| target_for(&edit.frame) == dest && edit.frame != frame)
        {
            continue;
        }
        update_matching_pending_target_values(&mut pending, frame, &value);
        pending.insert(
            row.row_id.clone(),
            PendingId3Edit {
                field: row.field.clone(),
                frame: frame.to_string(),
                value,
                source,
            },
        );
    }
    pending
}

pub fn auto_id3_source_value(row: &AlignedCompareRow) -> Option<(MetadataColumn, String)> {
    let rss = normalized_compare_value(row.rss_value.as_deref());
    let musicbrainz = normalized_compare_value(row.musicbrainz_value.as_deref());
    match (rss, musicbrainz) {
        (Some(rss), Some(musicbrainz)) if rss == musicbrainz => Some((MetadataColumn::Rss, rss)),
        (Some(_), Some(_)) => None,
        (Some(rss), None) => Some((MetadataColumn::Rss, rss)),
        (None, Some(musicbrainz)) => Some((MetadataColumn::MusicBrainz, musicbrainz)),
        (None, None) => None,
    }
}

pub fn update_matching_pending_target_values(
    pending: &mut BTreeMap<String, PendingId3Edit>,
    frame: &str,
    value: &str,
) {
    if matches!(
        id3_frame_base(frame),
        "WCOM" | "WOAR" | "COMM" | "TMCL" | "TIPL"
    ) {
        return;
    }
    let target = pending_id3_target_key(frame);
    for edit in pending.values_mut() {
        if pending_id3_target_key(&edit.frame) == target {
            edit.value = value.to_string();
        }
    }
}

pub fn id3_frame_group_key(frame_id: &str) -> &'static str {
    match id3_frame_base(frame_id) {
        "TALB" | "TIT1" | "TIT2" | "TIT3" | "TOAL" | "TPOS" | "TRCK" | "TSRC" | "TSST" => {
            "identification-release-structure"
        }
        "TCOM" | "TENC" | "TEXT" | "TIPL" | "TMCL" | "TOLY" | "TOPE" | "TPE1" | "TPE2" | "TPE3"
        | "TPE4" => "people-credits",
        "TBPM" | "TCON" | "TCOP" | "TDEN" | "TDLY" | "TDOR" | "TDRC" | "TDRL" | "TDTG" | "TFLT"
        | "TKEY" | "TLAN" | "TLEN" | "TMED" | "TMOO" | "TOFN" | "TOWN" | "TPRO" | "TPUB"
        | "TRSN" | "TRSO" | "TSOA" | "TSOP" | "TSOT" | "TSSE" | "TXXX" => {
            "descriptive-technical-rights-text"
        }
        "TDAT" | "TIME" | "TORY" | "TRDA" | "TYER" => "descriptive-technical-rights-text",
        "WCOM" | "WCOP" | "WOAF" | "WOAR" | "WOAS" | "WORS" | "WPAY" | "WPUB" | "WXXX" => {
            "url-link-frames"
        }
        "APIC" | "COMM" | "GEOB" | "PCNT" | "POPM" | "SYLT" | "USER" | "USLT" => {
            "lyrics-comments-artwork-user-facing-content"
        }
        "AENC" | "ENCR" | "GRID" | "LINK" | "PRIV" | "SIGN" | "UFID" => {
            "identity-linking-private-registration"
        }
        "ASPI" | "ETCO" | "EQU2" | "MLLT" | "POSS" | "RBUF" | "RVA2" | "RVRB" | "SEEK" | "SYTC" => {
            "timing-seeking-audio-analysis-playback-control"
        }
        "COMR" | "MCDI" | "OWNE" => "music-disc-acquisition-commerce",
        _ => "unknown",
    }
}

pub fn track_metadata_rows(
    track_context: &TrackContext,
    musicbrainz: Option<&MusicBrainzCandidate>,
    show_musicbrainz: bool,
) -> Vec<MetadataGridRow> {
    let track = &track_context.track;
    let mut rows = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
    push_track_metadata_row(
        &mut rows,
        "identification-release-structure",
        "Title",
        source_value_for_metadata_field("Title", track_context),
        musicbrainz_value_for_field("Title", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "people-credits",
        "Artist",
        source_value_for_metadata_field("Artist", track_context),
        musicbrainz_value_for_field("Artist", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "people-credits",
        "Album artist",
        source_value_for_metadata_field("Album artist", track_context),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "identification-release-structure",
        "Album/Feed",
        source_value_for_metadata_field("Album/Feed", track_context),
        musicbrainz_value_for_field("Album/Feed", musicbrainz),
    );
    let track_num = track.track_number.map(|n| n.to_string());
    let total_tracks = musicindex_total_tracks(track_context);
    let mb_track = musicbrainz_value_for_field("Track #", musicbrainz);
    let mb_total = musicbrainz_value_for_field("Total tracks", musicbrainz);
    push_track_metadata_row(
        &mut rows,
        "identification-release-structure",
        "Track #",
        format_track_slash_total(track_num.as_deref(), total_tracks.as_deref()),
        format_track_slash_total(mb_track.as_deref(), mb_total.as_deref()),
    );
    push_track_metadata_row(
        &mut rows,
        "descriptive-technical-rights-text",
        "Publisher",
        source_value_for_metadata_field("Publisher", track_context),
        musicbrainz_value_for_field("Publisher", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "identity-linking-private-registration",
        "RSS track guid",
        track.track_guid.clone(),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "identity-linking-private-registration",
        "RSS feed guid",
        track.feed_guid.clone().or_else(|| {
            track_context
                .feed
                .as_ref()
                .and_then(|feed| feed.feed_guid.clone())
        }),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "identity-linking-private-registration",
        "Nostr handle",
        track_nostr(track),
        None,
    );
    if let Some(feed) = track_context.feed.as_ref() {
        let feed_nostr_value = feed_nostr(feed);
        if normalized_compare_value(feed_nostr_value.as_deref())
            != normalized_compare_value(track_nostr(track).as_deref())
        {
            push_track_metadata_row(
                &mut rows,
                "identity-linking-private-registration",
                "RSS feed nostr handle",
                feed_nostr_value,
                None,
            );
        }
    }
    push_track_metadata_row(
        &mut rows,
        "url-link-frames",
        "Website",
        track_website(track),
        None,
    );
    if let Some(feed) = track_context.feed.as_ref() {
        let feed_website_value = feed_website(feed);
        if normalized_compare_value(feed_website_value.as_deref())
            != normalized_compare_value(track_website(track).as_deref())
        {
            push_track_metadata_row(
                &mut rows,
                "url-link-frames",
                "RSS feed website",
                feed_website_value,
                None,
            );
        }
    }
    push_track_metadata_row(
        &mut rows,
        "descriptive-technical-rights-text",
        "Release date",
        musicindex_release_date(track_context),
        musicbrainz_value_for_field("Release date", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "descriptive-technical-rights-text",
        "Release year",
        musicindex_release_date(track_context),
        musicbrainz_value_for_field("Release date", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "descriptive-technical-rights-text",
        "Duration",
        track.duration_secs.map(fmt_dur),
        musicbrainz_value_for_field("Duration", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "lyrics-comments-artwork-user-facing-content",
        "Artwork",
        track_artwork_url(track_context),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "lyrics-comments-artwork-user-facing-content",
        "Transcript",
        track_transcript_url(track),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "lyrics-comments-artwork-user-facing-content",
        "Transcript text",
        track_transcript_url(track),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "people-credits",
        "Contributors",
        track
            .source_contributors
            .as_deref()
            .and_then(musicindex_contributors_id3_value),
        musicbrainz_value_for_field("Contributors", musicbrainz),
    );
    if let Some(contributors) = track.source_contributors.as_deref() {
        for (field, frame, value) in contributor_id3_rows(contributors) {
            push_grouped_metadata_data_row(
                &mut rows,
                "people-credits",
                AlignedCompareRow {
                    row_id: compare_row_id(&field),
                    field,
                    rss_value: Some(value),
                    id3_value: None,
                    id3_frame: Some(frame.into()),
                    musicbrainz_value: None,
                    musicbrainz_key: None,
                    id3_status: ComparisonStatus::MissingTag,
                    musicbrainz_status: ComparisonStatus::MissingBoth,
                },
            );
        }
    }
    push_track_metadata_row(
        &mut rows,
        "music-disc-acquisition-commerce",
        "Value Routes",
        track
            .payment_routes
            .as_deref()
            .and_then(summarize_value_routes),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "descriptive-technical-rights-text",
        "RSS item pubdate",
        track_release_pubdate(track).filter(|item_pubdate| {
            musicindex_release_date(track_context).as_deref() != Some(item_pubdate)
        }),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "descriptive-technical-rights-text",
        "Description",
        source_value_for_metadata_field("Description", track_context),
        None,
    );

    if show_musicbrainz {
        if let Some(candidate) = musicbrainz {
            for row in musicbrainz_remainder_rows(candidate, track_context, None) {
                push_grouped_metadata_data_row(
                    &mut rows,
                    metadata_field_group_key(&row.field),
                    row,
                );
            }
        }
    }

    grouped_metadata_rows(rows)
}

pub fn musicindex_total_tracks(track_context: &TrackContext) -> Option<String> {
    let feed = track_context.feed.as_ref()?;
    feed.episode_count
        .map(|count| count.to_string())
        .or_else(|| feed.tracks.as_ref().map(|tracks| tracks.len().to_string()))
}

pub fn push_track_metadata_row(
    rows: &mut BTreeMap<&'static str, Vec<MetadataGridRow>>,
    group_key: &'static str,
    field: &str,
    rss_value: Option<String>,
    musicbrainz_value: Option<String>,
) {
    let rss_value = drop_placeholder_source_text(rss_value);
    let musicbrainz_status =
        compare_optional_values(rss_value.as_deref(), musicbrainz_value.as_deref());
    push_grouped_metadata_data_row(
        rows,
        group_key,
        AlignedCompareRow {
            row_id: compare_row_id(field),
            field: field.into(),
            rss_value,
            id3_value: None,
            id3_frame: id3_frame_hint(field).map(str::to_string),
            musicbrainz_value,
            musicbrainz_key: musicbrainz_key_for_field(field).map(str::to_string),
            id3_status: ComparisonStatus::MissingTag,
            musicbrainz_status,
        },
    );
}

pub fn aligned_compare_rows(
    result: &TagCompareResult,
    track_context: &TrackContext,
    musicbrainz: Option<&MusicBrainzCandidate>,
    show_musicbrainz: bool,
    expanded_id3_frame_groups: &BTreeSet<String>,
) -> Vec<MetadataGridRow> {
    let mut grouped_rows = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
    for row in &result.rows {
        let musicbrainz_value = musicbrainz_value_for_field(row.field, musicbrainz);
        let (rss_value, id3_value, mb_value, id3_compare_value) = if row.field == "Track #" {
            let total_rss = musicindex_total_tracks(track_context);
            let total_mb = musicbrainz_value_for_field("Total tracks", musicbrainz);
            let source_track = drop_placeholder_source_text(row.source_value.clone())
                .or_else(|| source_value_for_metadata_field(row.field, track_context));
            let id3_track_display =
                id3_value_for_field(row.field, result).or_else(|| row.tag_value.clone());
            let id3_track_compare = row
                .tag_value
                .clone()
                .or_else(|| id3_compare_value_for_field(row.field, result));
            (
                format_track_slash_total(source_track.as_deref(), total_rss.as_deref()),
                format_track_slash_total(
                    id3_track_display.as_deref(),
                    result.total_tracks.as_deref(),
                ),
                format_track_slash_total(musicbrainz_value.as_deref(), total_mb.as_deref()),
                format_track_slash_total(
                    id3_track_compare.as_deref(),
                    result.total_tracks.as_deref(),
                ),
            )
        } else {
            let id3_value =
                id3_value_for_field(row.field, result).or_else(|| row.tag_value.clone());
            let id3_compare_value = row
                .tag_value
                .clone()
                .or_else(|| id3_compare_value_for_field(row.field, result));
            (
                drop_placeholder_source_text(row.source_value.clone())
                    .or_else(|| source_value_for_metadata_field(row.field, track_context)),
                id3_value,
                musicbrainz_value,
                id3_compare_value,
            )
        };
        let id3_status = compare_id3_field_values(
            row.field,
            rss_value.as_deref(),
            id3_compare_value.as_deref(),
        );
        let musicbrainz_status = compare_optional_values(rss_value.as_deref(), mb_value.as_deref());
        push_grouped_metadata_data_row(
            &mut grouped_rows,
            metadata_field_group_key(row.field),
            AlignedCompareRow {
                row_id: compare_row_id(row.field),
                field: row.field.to_string(),
                rss_value,
                id3_value,
                id3_frame: id3_frame_hint(row.field).map(str::to_string),
                musicbrainz_status,
                musicbrainz_value: mb_value,
                musicbrainz_key: musicbrainz_key_for_field(row.field).map(str::to_string),
                id3_status,
            },
        );
    }

    let release_year_rss = musicindex_release_date(track_context)
        .or_else(|| comparison_source_value(result, "Release date"))
        .and_then(|value| release_year_from_value(&value));
    let release_year_id3 = id3_value_for_field("Release year", result);
    let release_year_musicbrainz = musicbrainz_value_for_field("Release date", musicbrainz)
        .and_then(|value| release_year_from_value(&value));
    let release_year_status =
        compare_optional_values(release_year_rss.as_deref(), release_year_id3.as_deref());
    let release_year_musicbrainz_status = compare_optional_values(
        release_year_rss.as_deref(),
        release_year_musicbrainz.as_deref(),
    );
    push_grouped_metadata_data_row(
        &mut grouped_rows,
        "descriptive-technical-rights-text",
        AlignedCompareRow {
            row_id: compare_row_id("Release year"),
            field: "Release year".into(),
            rss_value: release_year_rss,
            id3_value: release_year_id3,
            id3_frame: id3_frame_hint("Release year").map(str::to_string),
            musicbrainz_value: release_year_musicbrainz,
            musicbrainz_key: musicbrainz_key_for_field("Release date").map(str::to_string),
            id3_status: release_year_status,
            musicbrainz_status: release_year_musicbrainz_status,
        },
    );
    if let Some(tempo_id3) = id3_value_for_field("Tempo", result) {
        push_grouped_metadata_data_row(
            &mut grouped_rows,
            "timing-seeking-audio-analysis-playback-control",
            AlignedCompareRow {
                row_id: compare_row_id("Tempo"),
                field: "Tempo".into(),
                rss_value: None,
                id3_value: Some(tempo_id3),
                id3_frame: id3_frame_hint("Tempo").map(str::to_string),
                musicbrainz_value: None,
                musicbrainz_key: None,
                id3_status: ComparisonStatus::MissingSource,
                musicbrainz_status: ComparisonStatus::MissingBoth,
            },
        );
    }

    let contributors_rss = musicindex_contributors_id3_value(&result.contributors);
    let contributors_id3 = id3_value_for_field("Contributors", result);
    let contributors_id3_compare = id3_compare_value_for_field("Contributors", result);
    let contributors_musicbrainz = musicbrainz_value_for_field("Contributors", musicbrainz);
    let contributors_status = compare_id3_field_values(
        "Contributors",
        contributors_rss.as_deref(),
        contributors_id3_compare.as_deref(),
    );
    let contributors_musicbrainz_status = compare_optional_values(
        contributors_rss.as_deref(),
        contributors_musicbrainz.as_deref(),
    );
    push_grouped_metadata_data_row(
        &mut grouped_rows,
        "people-credits",
        AlignedCompareRow {
            row_id: compare_row_id("Contributors"),
            field: "Contributors".into(),
            rss_value: contributors_rss,
            id3_value: contributors_id3,
            id3_frame: id3_frame_hint("Contributors").map(str::to_string),
            musicbrainz_value: contributors_musicbrainz,
            musicbrainz_key: Some("track.artist-credit.name".into()),
            id3_status: contributors_status,
            musicbrainz_status: contributors_musicbrainz_status,
        },
    );
    for (field, frame, value) in contributor_id3_rows(&result.contributors) {
        let id3_value = id3_value_for_field(&field, result)
            .or_else(|| id3_values_for_frame(result, id3_frame_base(frame), &[]));
        let id3_status =
            compare_id3_field_values(&field, Some(value.as_str()), id3_value.as_deref());
        push_grouped_metadata_data_row(
            &mut grouped_rows,
            "people-credits",
            AlignedCompareRow {
                row_id: compare_row_id(&field),
                field,
                rss_value: Some(value),
                id3_value,
                id3_frame: Some(frame.into()),
                musicbrainz_value: None,
                musicbrainz_key: None,
                id3_status,
                musicbrainz_status: ComparisonStatus::MissingBoth,
            },
        );
    }
    let value_routes_rss = summarize_value_routes(&result.value_routes);
    let value_routes_id3 = id3_value_for_field("Value Routes", result);
    let value_routes_status =
        compare_optional_values(value_routes_rss.as_deref(), value_routes_id3.as_deref());
    push_grouped_metadata_data_row(
        &mut grouped_rows,
        "music-disc-acquisition-commerce",
        AlignedCompareRow {
            row_id: compare_row_id("Value Routes"),
            field: "Value Routes".into(),
            rss_value: value_routes_rss,
            id3_value: value_routes_id3,
            id3_frame: id3_frame_hint("Value Routes").map(str::to_string),
            musicbrainz_value: None,
            musicbrainz_key: None,
            id3_status: value_routes_status,
            musicbrainz_status: ComparisonStatus::MissingTag,
        },
    );
    let description_rss = source_value_for_metadata_field("Description", track_context);
    let description_id3 = id3_value_for_field("Description", result);
    let description_status =
        compare_optional_values(description_rss.as_deref(), description_id3.as_deref());
    push_grouped_metadata_data_row(
        &mut grouped_rows,
        "descriptive-technical-rights-text",
        AlignedCompareRow {
            row_id: compare_row_id("Description"),
            field: "Description".into(),
            rss_value: description_rss,
            id3_value: description_id3,
            id3_frame: id3_frame_hint("Description").map(str::to_string),
            musicbrainz_value: None,
            musicbrainz_key: None,
            id3_status: description_status,
            musicbrainz_status: ComparisonStatus::MissingBoth,
        },
    );
    let transcript_rss = track_transcript_url(&track_context.track);
    for field in ["Transcript", "Transcript text"] {
        let transcript_id3 = id3_value_for_field(field, result);
        let transcript_status =
            compare_id3_field_values(field, transcript_rss.as_deref(), transcript_id3.as_deref());
        push_grouped_metadata_data_row(
            &mut grouped_rows,
            "lyrics-comments-artwork-user-facing-content",
            AlignedCompareRow {
                row_id: compare_row_id(field),
                field: field.into(),
                rss_value: transcript_rss.clone(),
                id3_value: transcript_id3,
                id3_frame: id3_frame_hint(field).map(str::to_string),
                musicbrainz_value: None,
                musicbrainz_key: None,
                id3_status: transcript_status,
                musicbrainz_status: ComparisonStatus::MissingBoth,
            },
        );
    }

    if show_musicbrainz {
        if let Some(candidate) = musicbrainz {
            for row in musicbrainz_remainder_rows(candidate, track_context, Some(result)) {
                push_grouped_metadata_data_row(
                    &mut grouped_rows,
                    metadata_field_group_key(&row.field),
                    row,
                );
            }
        }
    }

    let mut rows = Vec::new();
    let aligned_frame_ids = aligned_id3_frame_ids(result, &grouped_rows);
    for &(group_key, label) in ID3V24_FRAME_GROUPS {
        let group_rows = grouped_rows.remove(group_key).unwrap_or_default();
        let unused = unused_id3v24_frames_for_group(result, group_key);
        let used = used_id3_fields_for_group(result, group_key, &aligned_frame_ids);
        let expanded = expanded_id3_frame_groups.contains(group_key);
        rows.push(metadata_group_row(
            label,
            Some(group_key),
            expanded,
            unused.len(),
        ));
        rows.extend(
            group_rows
                .into_iter()
                .filter(|row| expanded || !metadata_row_collapsed_by_default(row)),
        );
        rows.extend(
            used.into_iter()
                .filter(|field| expanded || !id3_field_collapsed_by_default(field))
                .map(used_id3_field_row),
        );
        rows.extend(unused.into_iter().map(id3_unused_frame_row));
    }

    rows.extend(grouped_metadata_rows(grouped_rows));

    rows
}

pub fn aligned_id3_frame_ids(
    result: &TagCompareResult,
    grouped_rows: &BTreeMap<&'static str, Vec<MetadataGridRow>>,
) -> BTreeSet<String> {
    let mut frame_ids = BTreeSet::new();
    for row in grouped_rows.values().flat_map(|rows| rows.iter()) {
        let MetadataGridRow::Data(row) = row else {
            continue;
        };
        if let Some(frame) = row.id3_frame.as_deref() {
            frame_ids.insert(pending_id3_target_key(frame));
        }
        for frame in grouped_id3_frame_keys(&row.field, result) {
            frame_ids.insert(frame);
        }
    }
    frame_ids
}

pub fn push_grouped_metadata_data_row(
    rows: &mut BTreeMap<&'static str, Vec<MetadataGridRow>>,
    group_key: &'static str,
    row: AlignedCompareRow,
) {
    rows.entry(group_key)
        .or_default()
        .push(metadata_data_row(row));
}

pub fn grouped_metadata_rows(
    mut grouped_rows: BTreeMap<&'static str, Vec<MetadataGridRow>>,
) -> Vec<MetadataGridRow> {
    let mut rows = Vec::new();
    for &(group_key, label) in ID3V24_FRAME_GROUPS {
        if let Some(group_rows) = grouped_rows.remove(group_key) {
            if !group_rows.is_empty() {
                rows.push(metadata_group_row(label, None, false, 0));
                rows.extend(group_rows);
            }
        }
    }

    for (_group_key, group_rows) in grouped_rows {
        if !group_rows.is_empty() {
            rows.push(metadata_group_row("Other metadata", None, false, 0));
            rows.extend(group_rows);
        }
    }

    rows
}

pub fn metadata_field_group_key(field: &str) -> &'static str {
    if performer_instrument_field(field).is_some() {
        return "people-credits";
    }
    match field {
        "Title"
        | "Album/Feed"
        | "Track #"
        | "MusicBrainz recording"
        | "MusicBrainz release"
        | "MusicBrainz release group"
        | "Media"
        | "Disc #"
        | "Disc subtitle"
        | "Total tracks"
        | "ISRC" => "identification-release-structure",
        "Artist" | "Contributors" | "Label" => "people-credits",
        "Publisher"
        | "Release date"
        | "Release year"
        | "RSS item pubdate"
        | "Release country"
        | "Release status"
        | "Release packaging"
        | "Barcode"
        | "Release note"
        | "Release type"
        | "Release secondary types"
        | "Track note"
        | "Duration"
        | "Description" => "descriptive-technical-rights-text",
        "Tempo" => "timing-seeking-audio-analysis-playback-control",
        "Artwork" | "Transcript" | "Transcript text" => {
            "lyrics-comments-artwork-user-facing-content"
        }
        "Website" | "RSS feed website" => "url-link-frames",
        "Nostr handle" | "RSS feed nostr handle" => "identity-linking-private-registration",
        "Value Routes" => "music-disc-acquisition-commerce",
        "Composer" | "Lyricist" | "Lead performer" | "Album artist" | "Conductor" | "Remixer"
        | "Original artist" | "Original lyricist" | "Involved musicians" => "people-credits",
        _ => "unknown",
    }
}

pub fn metadata_row_collapsed_by_default(row: &MetadataGridRow) -> bool {
    match row {
        MetadataGridRow::Group(_) => false,
        MetadataGridRow::Data(row) => {
            // Artwork, Transcript, Transcript text are always visible even when
            // the content group is collapsed — they become clickable to reveal
            // their content inline.
            if matches!(
                row.field.as_str(),
                "Artwork" | "Transcript" | "Transcript text"
            ) {
                return false;
            }
            row.id3_frame
                .as_deref()
                .is_some_and(id3_frame_collapsed_by_default)
        }
    }
}

/// Returns true for fields whose values should be shown inline on click
/// rather than always expanded.
pub fn metadata_field_is_expandable(field: &str) -> bool {
    matches!(
        field,
        "Contributors" | "Value Routes" | "Artwork" | "Transcript" | "Transcript text"
    )
}

pub fn id3_field_collapsed_by_default(field: &Id3Field) -> bool {
    id3_frame_collapsed_by_default(&field.frame_id)
}

fn id3_frame_collapsed_by_default(frame_label: &str) -> bool {
    matches!(id3_frame_base(frame_label), "SYLT" | "USLT")
}

pub fn musicbrainz_value_for_field(
    field: &str,
    candidate: Option<&MusicBrainzCandidate>,
) -> Option<String> {
    let candidate = candidate?;
    match field {
        "Title" => Some(candidate.title.clone()),
        "Artist" => candidate.artist.clone(),
        "Album/Feed" => candidate.release_title.clone(),
        "Track #" => candidate.track_number.clone(),
        "Total tracks" => candidate.total_tracks.map(|count| count.to_string()),
        "Publisher" => None,
        "Contributors" => candidate
            .track_artist
            .clone()
            .or_else(|| candidate.artist.clone()),
        "Website" | "RSS feed website" => join_values(&candidate.urls),
        "Release date" => candidate.release_date.clone(),
        "Duration" => candidate.track_length_ms.map(fmt_ms),
        _ => None,
    }
}

pub fn musicbrainz_key_for_field(field: &str) -> Option<&'static str> {
    match field {
        "Title" => Some("recording.title"),
        "Artist" => Some("artist-credit.name"),
        "Album/Feed" => Some("release.title"),
        "Track #" => Some("track.number/medium.track-count"),
        "Contributors" => Some("track.artist-credit.name"),
        "Website" | "RSS feed website" => Some("relation.url.resource"),
        "Release date" => Some("release.date"),
        "Duration" => Some("track.length"),
        _ => None,
    }
}

pub fn musicbrainz_remainder_rows(
    candidate: &MusicBrainzCandidate,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
) -> Vec<AlignedCompareRow> {
    let mut rows = Vec::new();
    let mut push = |field: &str, musicbrainz_key: &str, value: Option<String>| {
        push_musicbrainz_only_row(
            &mut rows,
            track_context,
            result,
            field,
            musicbrainz_key,
            value,
        );
    };

    push(
        "MusicBrainz recording",
        "recording.id",
        Some(candidate.recording_id.clone()),
    );
    push(
        "MusicBrainz release",
        "release.id",
        candidate.release_id.clone(),
    );
    push(
        "MusicBrainz release group",
        "release-group.id",
        candidate.release_group_id.clone(),
    );
    push(
        "Release country",
        "release.country",
        candidate.country.clone(),
    );
    push(
        "Release status",
        "release.status",
        candidate.release_status.clone(),
    );
    push(
        "Release packaging",
        "release.packaging",
        candidate.release_packaging.clone(),
    );
    push(
        "Barcode",
        "release.barcode",
        candidate.release_barcode.clone(),
    );
    push(
        "Release note",
        "release.disambiguation",
        candidate.release_disambiguation.clone(),
    );
    push(
        "Release type",
        "release-group.primary-type",
        candidate.release_group_type.clone(),
    );
    push(
        "Release secondary types",
        "release-group.secondary-types",
        join_values(&candidate.release_group_secondary_types),
    );
    push("Label", "label-info", join_values(&candidate.labels));
    push("Media", "medium.format", candidate.format.clone());
    push(
        "Disc #",
        "medium.position",
        candidate
            .medium_position
            .map(|position| position.to_string()),
    );
    push(
        "Disc subtitle",
        "medium.title",
        candidate.medium_title.clone(),
    );
    push(
        "Track note",
        "recording.disambiguation",
        candidate.track_disambiguation.clone(),
    );
    push("ISRC", "recording.isrcs", join_values(&candidate.isrcs));
    rows
}

pub fn push_musicbrainz_only_row(
    rows: &mut Vec<AlignedCompareRow>,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
    field: &str,
    musicbrainz_key: &str,
    value: Option<String>,
) {
    if normalized_compare_value(value.as_deref()).is_some() {
        let rss_value = musicbrainz_source_value_for_field(field, track_context, result);
        let id3_value = result.and_then(|result| id3_value_for_field(field, result));
        let id3_status = compare_optional_values(rss_value.as_deref(), id3_value.as_deref());
        let musicbrainz_status = compare_optional_values(rss_value.as_deref(), value.as_deref());
        rows.push(AlignedCompareRow {
            row_id: compare_row_id(field),
            field: field.into(),
            rss_value,
            id3_value,
            id3_frame: id3_frame_hint(field).map(str::to_string),
            musicbrainz_value: value,
            musicbrainz_key: Some(musicbrainz_key.into()),
            id3_status,
            musicbrainz_status,
        });
    }
}

pub fn musicbrainz_source_value_for_field(
    field: &str,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
) -> Option<String> {
    let track = &track_context.track;
    result
        .and_then(|result| {
            musicbrainz_equivalent_compare_field(field)
                .and_then(|compare_field| comparison_source_value(result, compare_field))
        })
        .or_else(|| match field {
            "MusicBrainz recording" => source_id_by_scheme(
                track.source_ids.as_deref(),
                &[
                    "musicbrainz_recordingid",
                    "musicbrainz_recording_id",
                    "musicbrainz_trackid",
                    "musicbrainz_track_id",
                ],
            ),
            "MusicBrainz release" => source_id_by_scheme(
                track.source_ids.as_deref(),
                &[
                    "musicbrainz_albumid",
                    "musicbrainz_album_id",
                    "musicbrainz_releaseid",
                    "musicbrainz_release_id",
                ],
            ),
            "MusicBrainz release group" => source_id_by_scheme(
                track.source_ids.as_deref(),
                &[
                    "musicbrainz_releasegroupid",
                    "musicbrainz_release_group_id",
                    "musicbrainz_release_groupid",
                ],
            ),
            "Barcode" => source_id_by_scheme(track.source_ids.as_deref(), &["barcode"]),
            "ISRC" => source_id_by_scheme(track.source_ids.as_deref(), &["isrc"]),
            "Duration" => track.duration_secs.map(fmt_dur),
            "Release year" => musicindex_release_date(track_context),
            _ => source_value_for_metadata_field(field, track_context),
        })
}

fn source_value_for_metadata_field(field: &str, track_context: &TrackContext) -> Option<String> {
    let track = &track_context.track;
    let feed = track_context.feed.as_ref();
    let value = match field {
        "Title" => drop_placeholder_source_text(track.title.clone())
            .or_else(|| drop_placeholder_source_text(track.name.clone()))
            .as_deref()
            .and_then(format_id3_title),
        "Artist" => drop_placeholder_source_text(track.track_artist.clone()),
        "Album artist" => {
            drop_placeholder_source_text(track.release_artist.clone()).or_else(|| {
                feed.and_then(|feed| drop_placeholder_source_text(feed.release_artist.clone()))
            })
        }
        "Album/Feed" => track
            .feed_title
            .clone()
            .and_then(|value| drop_placeholder_source_text(Some(value)))
            .or_else(|| {
                feed.and_then(|feed| {
                    drop_placeholder_source_text(feed.title.clone())
                        .or_else(|| drop_placeholder_source_text(feed.name.clone()))
                })
            }),
        "Track #" => track
            .track_number
            .map(|track_number| track_number.to_string()),
        "Publisher" | "Label" => drop_placeholder_source_text(track.publisher_text.clone())
            .or_else(|| {
                feed.and_then(|feed| drop_placeholder_source_text(feed.publisher_text.clone()))
            }),
        "RSS track guid" => track.track_guid.clone(),
        "RSS feed guid" => track
            .feed_guid
            .clone()
            .or_else(|| feed.and_then(|feed| feed.feed_guid.clone())),
        "Nostr handle" => track_nostr(track),
        "RSS feed nostr handle" => feed.and_then(feed_nostr),
        "Website" => track_website(track),
        "RSS feed website" => feed.and_then(feed_website),
        "Release date" | "Release year" => musicindex_release_date(track_context),
        "RSS item pubdate" => track_release_pubdate(track).filter(|item_pubdate| {
            musicindex_release_date(track_context).as_deref() != Some(item_pubdate)
        }),
        "Duration" => track.duration_secs.map(fmt_dur),
        "Artwork" => track_artwork_url(track_context),
        "Transcript" | "Transcript text" => track_transcript_url(track),
        "Description" => drop_placeholder_source_text(track.description.clone()).or_else(|| {
            feed.and_then(|feed| drop_placeholder_source_text(feed.description.clone()))
        }),
        "Contributors" => track
            .source_contributors
            .as_deref()
            .and_then(musicindex_contributors_id3_value),
        "Value Routes" => track
            .payment_routes
            .as_deref()
            .and_then(summarize_value_routes),
        _ => None,
    };
    drop_placeholder_source_text(value)
}

pub fn metadata_group_row(
    label: &str,
    key: Option<&str>,
    expanded: bool,
    unused_count: usize,
) -> MetadataGridRow {
    MetadataGridRow::Group(MetadataGroupRow {
        key: key.map(str::to_string),
        label: label.to_string(),
        expanded,
        unused_count,
    })
}

pub fn metadata_data_row(row: AlignedCompareRow) -> MetadataGridRow {
    MetadataGridRow::Data(row)
}

pub fn compare_row_id(field: &str) -> String {
    format!("compare:{}", field.to_ascii_lowercase().replace(' ', "-"))
}

pub fn id3_unused_frame_row(frame_id: &str) -> MetadataGridRow {
    MetadataGridRow::Data(AlignedCompareRow {
        row_id: compare_row_id(&format!("unused-id3-{frame_id}")),
        field: frame_id.to_string(),
        rss_value: None,
        id3_value: None,
        id3_frame: Some(frame_id.to_string()),
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingTag,
        musicbrainz_status: ComparisonStatus::MissingBoth,
    })
}

pub fn used_id3_field_row(field: &Id3Field) -> MetadataGridRow {
    MetadataGridRow::Data(AlignedCompareRow {
        row_id: compare_row_id(&format!("id3-{}", field.frame_id)),
        field: field.frame_id.clone(),
        rss_value: None,
        id3_value: Some(field.value.clone()),
        id3_frame: Some(field.frame_id.clone()),
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingTag,
        musicbrainz_status: ComparisonStatus::MissingBoth,
    })
}

pub fn unused_id3v24_frames_for_group(
    result: &TagCompareResult,
    group_key: &str,
) -> Vec<&'static str> {
    result
        .id3_fields
        .iter()
        .map(|field| id3_frame_base(&field.frame_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|frame_id| id3_frame_group_key(frame_id) == group_key)
        .collect::<Vec<_>>()
        .into_iter()
        .flat_map(|used_frame_id| {
            ID3V24_FRAME_IDS
                .iter()
                .find_map(|frame_id| {
                    (id3_frame_base(frame_id) == used_frame_id).then_some(*frame_id)
                })
                .into_iter()
        })
        .filter(|frame_id| {
            !result
                .id3_fields
                .iter()
                .any(|field| id3_frame_base(&field.frame_id) == *frame_id)
        })
        .collect()
}

pub fn used_id3_fields_for_group<'a>(
    result: &'a TagCompareResult,
    group_key: &str,
    aligned_frame_ids: &BTreeSet<String>,
) -> Vec<&'a Id3Field> {
    result
        .id3_fields
        .iter()
        .filter(|field| !id3_frame_is_summarized(&field.frame_id))
        .filter(|field| !aligned_frame_ids.contains(&pending_id3_target_key(&field.frame_id)))
        .filter(|field| id3_frame_group_key(&field.frame_id) == group_key)
        .collect()
}

// ID3 formatting

pub fn id3_value_for_field(field: &str, result: &TagCompareResult) -> Option<String> {
    if let Some(instrument) = performer_instrument_field(field) {
        return id3_involved_people_value(result, "TMCL", &instrument);
    }

    if field == "Tempo" {
        return id3_tempo_values(result);
    }

    if let Some(value) = id3_sort_order_values(field, result) {
        return Some(value);
    }

    if field == "Contributors" {
        return id3_contributor_values(result);
    }

    let frame_label = id3_frame_hint(field)?;
    let frame_id = id3_frame_base(frame_label);
    if frame_id == "TLEN" {
        return id3_values_for_frame(result, frame_id, &[])
            .map(|value| value.parse::<i64>().ok().map(fmt_ms).unwrap_or(value));
    }
    if frame_id == "TXXX" {
        return id3_values_for_frame(result, frame_id, id3_txxx_needles(field));
    }
    if matches!(frame_id, "COMM" | "USLT" | "SYLT") {
        return id3_values_for_frame(result, frame_id, id3_descriptor_needles(field));
    }

    let needles = if field == "MusicBrainz recording" {
        &["musicbrainz"][..]
    } else {
        &[][..]
    };
    id3_values_for_frame(result, frame_id, needles)
}

pub fn id3_compare_value_for_field(field: &str, result: &TagCompareResult) -> Option<String> {
    if let Some(value) = musicbrainz_equivalent_compare_field(field)
        .and_then(|compare_field| comparison_tag_value(result, compare_field))
    {
        return Some(value);
    }

    if let Some(instrument) = performer_instrument_field(field) {
        return id3_involved_people_value(result, "TMCL", &instrument);
    }

    if field == "Contributors" {
        return id3_contributor_values(result);
    }

    let frame_label = id3_frame_hint(field)?;
    let frame_id = id3_frame_base(frame_label);
    if frame_id == "TLEN" {
        return id3_values_for_frame(result, frame_id, &[])
            .map(|value| value.parse::<i64>().ok().map(fmt_ms).unwrap_or(value));
    }
    if frame_id == "TXXX" {
        return id3_values_for_frame(result, frame_id, id3_txxx_needles(field));
    }
    if matches!(frame_id, "COMM" | "USLT" | "SYLT") {
        return id3_values_for_frame(result, frame_id, id3_descriptor_needles(field));
    }

    let needles = if field == "MusicBrainz recording" {
        &["musicbrainz"][..]
    } else {
        &[][..]
    };
    id3_values_for_frame(result, frame_id, needles)
}

pub fn performer_instrument_field(field: &str) -> Option<String> {
    field
        .strip_prefix("Performer [")?
        .strip_suffix(']')
        .map(str::trim)
        .filter(|instrument| !instrument.is_empty())
        .map(ToOwned::to_owned)
}

pub fn id3_involved_people_value(
    result: &TagCompareResult,
    frame_id: &str,
    involvement: &str,
) -> Option<String> {
    let involvement = involvement.to_ascii_lowercase();
    let values = result
        .id3_fields
        .iter()
        .filter(|field| id3_frame_base(&field.frame_id) == frame_id)
        .flat_map(|field| field.value.split(" / "))
        .filter_map(|entry| {
            let (role, name) = involved_people_entry(entry)?;
            role.eq_ignore_ascii_case(&involvement).then_some(name)
        })
        .collect::<Vec<_>>();
    join_values(&values)
}

fn involved_people_entry(entry: &str) -> Option<(String, String)> {
    let entry = entry.trim();
    let (left, right) = entry
        .split_once(": ")
        .or_else(|| entry.split_once(':'))
        .or_else(|| entry.split_once(" - "))?;
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let left_is_role = instrument_role(left).is_some() || involved_people_role(left) == left;
    let right_is_role = instrument_role(right).is_some() || involved_people_role(right) == right;
    if left_is_role || !right_is_role {
        Some((left.to_string(), right.to_string()))
    } else {
        Some((right.to_string(), left.to_string()))
    }
}

pub fn format_drag_value_for_id3v24(
    frame_label: &str,
    target_field: &str,
    existing_value: Option<&str>,
    value: &str,
) -> Option<String> {
    let value = sanitize_id3_text(value);
    if value.is_empty() {
        return None;
    }
    let frame_id = id3_frame_base(frame_label);
    match frame_id {
        "TIT2" => format_id3_title(&value),
        "TMCL" => format_tmcl_value(target_field, &value),
        "TXXX" | "UFID" => Some(value),
        "COMM" | "USLT" | "SYLT" => Some(value),
        "WXXX" => format_id3_url(&value),
        "TRCK" => format_slash_number_frame(target_field, existing_value, &value),
        "TPOS" => format_slash_number_frame(target_field, existing_value, &value),
        "TLEN" => format_id3_duration_ms(&value),
        "APIC" => format_id3_url(&value),
        "TYER" => format_id3_timestamp(&value).map(|value| value.chars().take(4).collect()),
        "TDRC" | "TDRL" | "TDOR" => format_id3_timestamp(&value),
        id if id.starts_with('W') => format_id3_url(&value),
        _ => Some(value),
    }
}

pub fn format_id3_title(value: &str) -> Option<String> {
    let value = value
        .trim_start()
        .strip_prefix("- ")
        .map(str::trim_start)
        .unwrap_or(value)
        .to_string();
    (!value.trim().is_empty()).then_some(value)
}

pub fn format_tmcl_value(target_field: &str, value: &str) -> Option<String> {
    let instrument = performer_instrument_field(target_field)?;
    let values = value
        .split(" / ")
        .filter_map(|name| normalized_compare_value(Some(name)))
        .map(|name| format!("{instrument}: {name}"))
        .collect::<Vec<_>>();
    join_values(&values)
}

pub fn format_source_value_for_id3v24(
    frame_label: &str,
    target_field: &str,
    source: MetadataColumn,
    existing_value: Option<&str>,
    value: &str,
) -> Option<String> {
    let prepared = if source == MetadataColumn::Rss
        && id3_frame_base(frame_label) == "WOAR"
        && target_field.to_ascii_lowercase().contains("website")
    {
        format!("download for free (url, forward): {value}")
    } else {
        value.to_string()
    };
    format_drag_value_for_id3v24(frame_label, target_field, existing_value, &prepared)
}

pub fn format_slash_number_frame(
    target_field: &str,
    existing_value: Option<&str>,
    value: &str,
) -> Option<String> {
    let (value_position, value_total) = split_slash_number(value);
    let value_position = value_position?;
    let existing = existing_value
        .and_then(|value| normalized_compare_value(Some(value)))
        .unwrap_or_default();
    let (existing_position, existing_total) = split_slash_number(&existing);

    match target_field {
        "Total tracks" => {
            let total = value_total.unwrap_or(value_position);
            existing_position.map(|position| format!("{position}/{total}"))
        }
        "Disc total" => {
            let total = value_total.unwrap_or(value_position);
            existing_position.map(|position| format!("{position}/{total}"))
        }
        _ => {
            if let Some(total) = existing_total.or(value_total) {
                Some(format!("{value_position}/{total}"))
            } else {
                Some(value_position)
            }
        }
    }
}

pub fn split_slash_number(value: &str) -> (Option<String>, Option<String>) {
    let Some((position, total)) = value.split_once('/') else {
        return (first_unsigned_number(value), None);
    };
    (
        first_unsigned_number(position),
        first_unsigned_number(total),
    )
}

pub fn first_unsigned_number(value: &str) -> Option<String> {
    let digits = value
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    (!digits.is_empty()).then_some(digits)
}

pub fn format_id3_timestamp(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.len() >= 4 && value.chars().take(4).all(|ch| ch.is_ascii_digit()) {
        return Some(value.to_string());
    }

    chrono::NaiveDate::parse_from_str(value, "%b %e, %Y")
        .ok()
        .map(|date| date.format("%Y-%m-%d").to_string())
}

pub fn format_id3_duration_ms(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        return Some(value.to_string());
    }

    let parts = value.split(':').collect::<Vec<_>>();
    let seconds = match parts.as_slice() {
        [minutes, seconds] => minutes
            .trim()
            .parse::<i64>()
            .ok()?
            .checked_mul(60)?
            .checked_add(seconds.trim().parse::<i64>().ok()?)?,
        [hours, minutes, seconds] => hours
            .trim()
            .parse::<i64>()
            .ok()?
            .checked_mul(3600)?
            .checked_add(minutes.trim().parse::<i64>().ok()?.checked_mul(60)?)?
            .checked_add(seconds.trim().parse::<i64>().ok()?)?,
        _ => return None,
    };
    seconds
        .checked_mul(1000)
        .map(|duration| duration.to_string())
}

pub fn format_id3_url(value: &str) -> Option<String> {
    let url = value.split('·').next().map(str::trim).unwrap_or(value);
    (!url.is_empty()).then(|| url.to_string())
}

pub fn id3v24_drag_copy_frame_is_writable(frame_label: &str) -> bool {
    id3v24_edit_label_is_writable(frame_label)
}

pub fn pending_id3_conflict_descriptions(edits: &BTreeMap<String, PendingId3Edit>) -> Vec<String> {
    let mut by_target = BTreeMap::<String, Vec<&PendingId3Edit>>::new();
    for edit in edits.values() {
        by_target
            .entry(pending_id3_effective_target_key(edit))
            .or_default()
            .push(edit);
    }

    by_target
        .into_iter()
        .filter_map(|(target, edits)| {
            if edits
                .first()
                .is_some_and(|edit| pending_id3_values_are_mergeable(&edit.frame))
            {
                return None;
            }
            let values = edits
                .iter()
                .filter_map(|edit| normalized_compare_value(Some(&edit.value)))
                .collect::<BTreeSet<_>>();
            if values.len() < 2 {
                return None;
            }
            let fields = edits
                .iter()
                .map(|edit| edit.field.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("{target} ({fields})"))
        })
        .collect()
}

pub fn pending_id3_edits_for_apply(edits: &BTreeMap<String, PendingId3Edit>) -> Vec<Id3v24Edit> {
    let mut by_target = BTreeMap::<String, Id3v24Edit>::new();
    for edit in edits.values() {
        let target = pending_id3_effective_target_key(edit);
        if pending_id3_values_are_mergeable(&edit.frame) {
            if let Some(existing) = by_target.get_mut(&target) {
                existing.value = merge_joined_values(&existing.value, &edit.value);
                continue;
            }
        }
        by_target.entry(target).or_insert_with(|| Id3v24Edit {
            frame_label: edit.frame.clone(),
            value: edit.value.clone(),
        });
    }
    by_target.into_values().collect()
}

pub fn pending_id3_values_are_mergeable(frame_label: &str) -> bool {
    matches!(id3_frame_base(frame_label), "TMCL" | "TIPL")
}

pub fn merge_joined_values(existing: &str, incoming: &str) -> String {
    let mut seen = BTreeSet::new();
    existing
        .split(" / ")
        .chain(incoming.split(" / "))
        .filter_map(|value| normalized_compare_value(Some(value)))
        .filter(|value| seen.insert(value.to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join(" / ")
}

pub fn pending_id3_effective_target_key(edit: &PendingId3Edit) -> String {
    let frame_id = id3_frame_base(&edit.frame).to_ascii_uppercase();
    match frame_id.as_str() {
        "WCOM" | "WOAR" | "COMM" => {
            let value = normalized_compare_value(Some(&edit.value)).unwrap_or_default();
            format!(
                "{}:{}",
                pending_id3_target_key(&edit.frame),
                value.to_ascii_lowercase()
            )
        }
        _ => pending_id3_target_key(&edit.frame),
    }
}

pub fn pending_id3_target_key(frame_label: &str) -> String {
    let frame_id = id3_frame_base(frame_label).to_ascii_uppercase();
    match frame_id.as_str() {
        "TXXX" | "WXXX" | "UFID" | "COMM" | "USLT" | "SYLT" => {
            let descriptor = normalized_id3_descriptor_key(frame_label);
            format!("{frame_id}:{descriptor}")
        }
        _ => frame_id,
    }
}

/// Where a given ID3 frame would land in `format`'s tag scheme.
///
/// Returns `None` if the format has no destination — the caller should not
/// auto-stage an edit it can't persist (e.g. TIPL/TMCL/TLEN on Vorbis).
///
/// MP4 always returns `Some` because we fall back to a freeform `----:`
/// atom for any unknown ID3 frame.
pub fn frame_destination_for_format(
    frame_label: &str,
    format: crate::audio_format::AudioFormat,
) -> Option<String> {
    use crate::audio_format::AudioFormat;
    use crate::tag_field::TagFieldId;

    let field = TagFieldId::from_id3_label(frame_label);
    match format {
        AudioFormat::Mp3 => Some(pending_id3_target_key(frame_label)),
        AudioFormat::Flac | AudioFormat::OggVorbis | AudioFormat::OggOpus => field.vorbis_key(),
        AudioFormat::Mp4 => Some(
            field
                .mp4_atom()
                .map(str::to_string)
                .unwrap_or_else(|| format!("----:com.apple.iTunes:{}", frame_label)),
        ),
        AudioFormat::Wav => None,
    }
}

pub fn sanitize_id3_text(value: &str) -> String {
    value.replace('\0', " ").trim().to_string()
}

pub fn id3_values_for_frame(
    result: &TagCompareResult,
    frame_id: &str,
    needles: &[&str],
) -> Option<String> {
    let values = result
        .id3_fields
        .iter()
        .filter(|field| id3_frame_base(&field.frame_id) == frame_id)
        .filter(|field| {
            let searchable = format!("{} {}", field.frame_id, field.value);
            needles_match(&searchable, needles)
        })
        .map(|field| field.value.clone())
        .collect::<Vec<_>>();
    join_values(&values)
}

fn id3_values_for_target_key(result: &TagCompareResult, target_key: &str) -> Option<String> {
    let values = result
        .id3_fields
        .iter()
        .filter(|field| pending_id3_target_key(&field.frame_id) == target_key)
        .filter_map(|field| normalized_compare_value(Some(&field.value)))
        .collect::<Vec<_>>();
    join_values(&values)
}

fn id3_grouped_frame_lines(result: &TagCompareResult, frame_labels: &[&str]) -> Vec<String> {
    frame_labels
        .iter()
        .filter_map(|frame_label| {
            id3_values_for_target_key(result, &pending_id3_target_key(frame_label))
                .map(|value| format!("{frame_label}: {value}"))
        })
        .collect()
}

pub fn id3_tempo_values(result: &TagCompareResult) -> Option<String> {
    join_line_values(&id3_grouped_frame_lines(result, tempo_id3_frame_labels()))
}

pub fn tempo_id3_frame_labels() -> &'static [&'static str] {
    &["TBPM", "TXXX:IBPM", "TXXX:tempo", "TXXX:bpm"]
}

pub fn grouped_id3_frame_keys(field: &str, result: &TagCompareResult) -> Vec<String> {
    match field {
        "Contributors" => contributor_grouped_id3_frame_keys(result),
        _ => grouped_id3_frame_labels(field)
            .into_iter()
            .flat_map(|frames| frames.iter())
            .map(|frame| pending_id3_target_key(frame))
            .collect(),
    }
}

pub fn grouped_id3_frame_labels(field: &str) -> Option<&'static [&'static str]> {
    match field {
        "Tempo" => Some(tempo_id3_frame_labels()),
        "Title" => Some(&["TIT2", "TSOT"]),
        "Artist" => Some(&["TPE1", "TSOP"]),
        "Album/Feed" => Some(&["TALB", "TSOA"]),
        "Contributors" => Some(&[
            "TXXX:MusicIndex Contributors",
            "TXXX:MUSICIANCREDITS",
            "TXXX:Musician Credits",
            "TCOM",
            "TEXT",
            "TIPL",
            "TMCL",
            "TPE1",
            "TPE2",
            "TPE3",
            "TPE4",
            "TOPE",
            "TOLY",
        ]),
        _ => None,
    }
}

pub fn id3_sort_order_values(field: &str, result: &TagCompareResult) -> Option<String> {
    let frame_labels = match field {
        "Title" => ["TIT2", "TSOT"].as_slice(),
        "Artist" => ["TPE1", "TSOP"].as_slice(),
        "Album/Feed" => ["TALB", "TSOA"].as_slice(),
        _ => return None,
    };
    let sort_targets = frame_labels
        .iter()
        .skip(1)
        .map(|frame| pending_id3_target_key(frame))
        .collect::<BTreeSet<_>>();
    let has_sort_order = result
        .id3_fields
        .iter()
        .any(|field| sort_targets.contains(&pending_id3_target_key(&field.frame_id)));
    has_sort_order.then(|| join_line_values(&id3_grouped_frame_lines(result, frame_labels)))?
}

pub fn id3_contributor_values(result: &TagCompareResult) -> Option<String> {
    let mut seen = BTreeSet::<(String, String)>::new();
    let values = result
        .id3_fields
        .iter()
        .flat_map(id3_contributor_entries_for_field)
        .filter(|(role, name)| seen.insert((role.to_ascii_lowercase(), name.to_ascii_lowercase())))
        .map(|(role, name)| format!("{role}: {name}"))
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join(" / "))
}

fn contributor_grouped_id3_frame_keys(result: &TagCompareResult) -> Vec<String> {
    result
        .id3_fields
        .iter()
        .filter(|field| id3_field_is_contributor_related(field))
        .map(|field| pending_id3_target_key(&field.frame_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn id3_field_is_contributor_related(field: &Id3Field) -> bool {
    match id3_frame_base(&field.frame_id) {
        "TCOM" | "TEXT" | "TIPL" | "TMCL" | "TPE1" | "TPE2" | "TPE3" | "TPE4" | "TOPE" | "TOLY" => {
            true
        }
        "TXXX" => {
            let descriptor = normalized_id3_descriptor_key(&field.frame_id);
            descriptor == "musicindex contributors"
                || (descriptor.contains("musician") && descriptor.contains("credit"))
        }
        _ => false,
    }
}

fn id3_contributor_entries_for_field(field: &Id3Field) -> Vec<(String, String)> {
    match id3_frame_base(&field.frame_id) {
        "TXXX" if id3_field_is_contributor_related(field) => {
            contributor_entries_from_serialized(&field.value)
        }
        "TCOM" => contributor_entries_for_people_frame(&field.value, "composer"),
        "TEXT" => contributor_entries_for_people_frame(&field.value, "lyricist"),
        "TPE1" => contributor_entries_for_people_frame(&field.value, "musician"),
        "TPE2" => contributor_entries_for_people_frame(&field.value, "album artist"),
        "TPE3" => contributor_entries_for_people_frame(&field.value, "conductor"),
        "TPE4" => contributor_entries_for_people_frame(&field.value, "remixer"),
        "TOPE" => contributor_entries_for_people_frame(&field.value, "original artist"),
        "TOLY" => contributor_entries_for_people_frame(&field.value, "original lyricist"),
        "TIPL" | "TMCL" => field
            .value
            .split(" / ")
            .filter_map(involved_people_entry)
            .filter_map(|(role, name)| contributor_entry(&role, &name))
            .collect(),
        _ => Vec::new(),
    }
}

fn contributor_entries_for_people_frame(value: &str, role: &str) -> Vec<(String, String)> {
    value
        .split(" / ")
        .flat_map(split_joined_metadata_values)
        .filter_map(|name| contributor_entry(role, &name))
        .collect()
}

fn contributor_entries_from_serialized(value: &str) -> Vec<(String, String)> {
    value
        .split(" / ")
        .filter_map(involved_people_entry)
        .filter_map(|(role, name)| contributor_entry(&role, &name))
        .collect()
}

fn contributor_entry(role: &str, name: &str) -> Option<(String, String)> {
    let role = normalized_contributor_role(role)?;
    let name = normalized_compare_value(Some(name))?;
    Some((role, name))
}

pub fn id3_frame_base(frame_label: &str) -> &str {
    frame_label
        .split_once(':')
        .map_or(frame_label.trim(), |(base, _)| base.trim())
}

fn normalized_id3_descriptor_key(frame_label: &str) -> String {
    frame_label
        .split_once(':')
        .map_or("", |(_, descriptor)| descriptor)
        .chars()
        .map(|ch| {
            if ch == '\0' || ch.is_control() {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

pub fn needles_match(value: &str, needles: &[&str]) -> bool {
    if needles.is_empty() {
        return true;
    }
    let value = value.to_ascii_lowercase();
    needles
        .iter()
        .all(|needle| value.contains(&needle.to_ascii_lowercase()))
}

pub fn id3_txxx_needles(field: &str) -> &'static [&'static str] {
    match field {
        "RSS feed guid" => &["musicindex", "feed", "guid"],
        "RSS track guid" => &["musicindex", "track", "guid"],
        "MusicBrainz release" => &["musicbrainz", "album", "id"],
        "MusicBrainz release group" => &["musicbrainz", "release", "group", "id"],
        "Release country" => &["musicbrainz", "album", "release", "country"],
        "Release status" => &["musicbrainz", "album", "status"],
        "Barcode" => &["barcode"],
        "Release type" | "Release secondary types" => &["musicbrainz", "album", "type"],
        "Publisher" => &["v4v", "publisher"],
        "Nostr handle" | "RSS feed nostr handle" => &["nostr"],
        "Contributors" => &["musicindex", "contributors"],
        "Value Routes" => &["musicindex", "value", "routes"],
        _ => &[],
    }
}

pub fn id3_descriptor_needles(field: &str) -> &'static [&'static str] {
    match field {
        "Description" => &["musicindex", "description"],
        "Transcript" | "Transcript text" => &["musicindex", "transcript"],
        _ => &[],
    }
}

pub fn musicbrainz_equivalent_compare_field(field: &str) -> Option<&'static str> {
    match field {
        "Title" => Some("Title"),
        "Artist" => Some("Artist"),
        "Track #" => Some("Track #"),
        "Label" => Some("Publisher"),
        "Website" => Some("Website"),
        _ => None,
    }
}

pub fn comparison_source_value(result: &TagCompareResult, field: &str) -> Option<String> {
    result
        .rows
        .iter()
        .find(|row| row.field == field)
        .and_then(|row| drop_placeholder_source_text(row.source_value.clone()))
}

pub fn comparison_tag_value(result: &TagCompareResult, field: &str) -> Option<String> {
    result
        .rows
        .iter()
        .find(|row| row.field == field)
        .and_then(|row| row.tag_value.clone())
}

pub fn source_id_by_scheme(ids: Option<&[SourceEntityId]>, schemes: &[&str]) -> Option<String> {
    let ids = ids?;
    ids.iter().find_map(|id| {
        let scheme = id.scheme.as_deref()?.to_ascii_lowercase();
        if schemes.iter().any(|candidate| scheme == *candidate) {
            id.value.clone()
        } else {
            None
        }
    })
}

pub fn compare_optional_values(source: Option<&str>, target: Option<&str>) -> ComparisonStatus {
    let source = normalized_compare_value(source);
    let target = normalized_compare_value(target);
    match (&source, &target) {
        (Some(source), Some(target)) if source == target => ComparisonStatus::Match,
        (Some(_), Some(_)) => ComparisonStatus::Different,
        (Some(_), None) => ComparisonStatus::MissingTag,
        (None, Some(_)) => ComparisonStatus::MissingSource,
        (None, None) => ComparisonStatus::MissingBoth,
    }
}

pub fn compare_id3_field_values(
    field: &str,
    source: Option<&str>,
    target: Option<&str>,
) -> ComparisonStatus {
    if id3_presence_satisfies_field(field) {
        return match (
            normalized_compare_value(source),
            normalized_compare_value(target),
        ) {
            (Some(_), Some(_)) => ComparisonStatus::Match,
            (Some(_), None) => ComparisonStatus::MissingTag,
            (None, Some(_)) => ComparisonStatus::MissingSource,
            (None, None) => ComparisonStatus::MissingBoth,
        };
    }
    let source = normalized_field_compare_value(field, source);
    let target = normalized_field_compare_value(field, target);
    match (&source, &target) {
        (Some(source), Some(target)) if source == target => ComparisonStatus::Match,
        (Some(_), Some(_)) => ComparisonStatus::Different,
        (Some(_), None) => ComparisonStatus::MissingTag,
        (None, Some(_)) => ComparisonStatus::MissingSource,
        (None, None) => ComparisonStatus::MissingBoth,
    }
}

pub fn id3_presence_satisfies_field(field: &str) -> bool {
    matches!(field, "Artwork" | "Transcript" | "Transcript text")
}

pub fn normalized_field_compare_value(field: &str, value: Option<&str>) -> Option<String> {
    let value = normalized_compare_value(value)?;
    match field {
        "Release date" | "RSS item pubdate" => normalized_date_compare_value(&value),
        "Release year" => release_year_from_value(&value),
        "Website" | "RSS feed website" => normalized_url_compare_value(&value),
        "Contributors" => normalized_contributors_compare_value(&value),
        "Artist" | "Album artist" | "Lead performer" | "Composer" | "Lyricist" | "Conductor"
        | "Remixer" | "Original artist" | "Original lyricist" => {
            normalized_people_compare_value(&value)
        }
        field if performer_instrument_field(field).is_some() => {
            normalized_people_compare_value(&value)
        }
        _ => Some(value),
    }
}

fn normalized_date_compare_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() >= 10
        && value.chars().take(4).all(|ch| ch.is_ascii_digit())
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
    {
        return Some(value[..10].to_string());
    }
    if value.len() == 4 && value.chars().all(|ch| ch.is_ascii_digit()) {
        return Some(value.to_string());
    }
    chrono::NaiveDate::parse_from_str(value, "%b %e, %Y")
        .ok()
        .map(|date| date.format("%Y-%m-%d").to_string())
        .or_else(|| Some(value.to_string()))
}

fn normalized_url_compare_value(value: &str) -> Option<String> {
    Some(
        embedded_url(value)
            .unwrap_or_else(|| value.trim().to_string())
            .trim_end_matches('/')
            .to_ascii_lowercase(),
    )
}

fn normalized_people_compare_value(value: &str) -> Option<String> {
    let mut values = split_joined_metadata_values(value)
        .into_iter()
        .flat_map(|part| {
            part.split(" / ")
                .filter_map(|value| {
                    let value = involved_people_entry(value)
                        .map_or_else(|| value.trim().to_string(), |(_role, name)| name);
                    normalized_person_compare_key(&value)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        normalized_person_compare_key(value)
    } else {
        values.sort_unstable();
        values.dedup();
        Some(values.join("/"))
    }
}

fn normalized_contributors_compare_value(value: &str) -> Option<String> {
    let mut grouped = BTreeMap::<String, BTreeSet<String>>::new();
    for (role, name) in contributor_entries_from_serialized(value) {
        let Some(name_key) = normalized_person_compare_key(&name) else {
            continue;
        };
        let Some(role_key) = normalized_compare_value(Some(&role)) else {
            continue;
        };
        grouped.entry(name_key).or_default().insert(role_key);
    }
    if grouped.is_empty() {
        return normalized_compare_value(Some(value));
    }
    Some(
        grouped
            .into_iter()
            .map(|(name, roles)| {
                format!("{name}:{}", roles.into_iter().collect::<Vec<_>>().join(","))
            })
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn normalized_contributor_role(role: &str) -> Option<String> {
    let role = role.trim();
    if role.is_empty() {
        return None;
    }
    if let Some(instrument) = instrument_role(role) {
        return Some(instrument.to_string());
    }
    let role = role.to_ascii_lowercase();
    let normalized = match role.as_str() {
        role if role.contains("original lyric") => "original lyricist",
        role if role.contains("original artist") => "original artist",
        role if role.contains("composer") || role.contains("composed") => "composer",
        role if role.contains("lyric") || role.contains("text writer") || role == "writer" => {
            "lyricist"
        }
        role if role.contains("album artist")
            || role.contains("band")
            || role.contains("group") =>
        {
            "album artist"
        }
        role if role.contains("conductor") => "conductor",
        role if role.contains("remix") => "remixer",
        role if role.contains("master") => "mastering engineer",
        role if role.contains("mix") => "mix engineer",
        role if role.contains("engineer") => "engineer",
        role if role.contains("producer") => "producer",
        role if role.contains("arranger") => "arranger",
        role if role == "musician"
            || role.contains("lead")
            || role.contains("performer")
            || role.contains("artist") =>
        {
            "musician"
        }
        _ => role.as_str(),
    };
    Some(normalized.to_string())
}

fn normalized_person_compare_key(value: &str) -> Option<String> {
    let value = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    (!value.is_empty()).then_some(value)
}

pub fn release_year_from_value(value: &str) -> Option<String> {
    let year = value
        .chars()
        .collect::<Vec<_>>()
        .windows(4)
        .find_map(|window| {
            window
                .iter()
                .all(|ch| ch.is_ascii_digit())
                .then(|| window.iter().collect::<String>())
        })?;
    Some(year)
}

// Value summary helpers

pub fn summarize_contributors(contributors: &[Contributor]) -> Option<String> {
    if contributors.is_empty() {
        return None;
    }
    let mut by_name = BTreeMap::<String, Vec<String>>::new();
    for contributor in contributors {
        let Some(name) = contributor
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        let role = contributor.role.as_deref().map(str::trim).unwrap_or("");
        if role.is_empty() {
            by_name.entry(name.to_string()).or_default();
        } else {
            by_name
                .entry(name.to_string())
                .or_default()
                .push(role.to_string());
        }
    }
    Some(
        by_name
            .into_iter()
            .map(|(name, roles)| {
                if roles.is_empty() {
                    name
                } else {
                    format!("{name}: {}", roles.join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join(" · "),
    )
}

pub fn musicindex_contributors_id3_value(contributors: &[Contributor]) -> Option<String> {
    if contributors.is_empty() {
        return None;
    }
    let values = contributors
        .iter()
        .filter_map(|contributor| {
            let name = contributor
                .name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .filter(|name| !source_text_is_placeholder(name))?;
            let role = contributor
                .role
                .as_deref()
                .map(str::trim)
                .filter(|role| !source_text_is_placeholder(role))
                .unwrap_or("contributor");
            Some(format!("{role}: {name}"))
        })
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join(" / "))
}

pub fn contributor_id3_rows(contributors: &[Contributor]) -> Vec<(String, &'static str, String)> {
    let mut grouped = BTreeMap::<(String, &'static str), Vec<String>>::new();
    for contributor in contributors {
        let Some(name) = contributor
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .filter(|name| !source_text_is_placeholder(name))
        else {
            continue;
        };
        let role_raw = contributor.role.as_deref().unwrap_or("").trim();
        let role = if source_text_is_placeholder(role_raw) {
            ""
        } else {
            role_raw
        };
        if let Some(instrument) = instrument_role(role) {
            grouped
                .entry((format!("Performer [{instrument}]"), "TMCL"))
                .or_default()
                .push(name.to_string());
            continue;
        }
        let role_key = role.to_ascii_lowercase();
        let Some(target) = contributor_role_target(&role_key) else {
            continue;
        };
        if target.1 == "TIPL" {
            grouped
                .entry((target.0.to_string(), target.1))
                .or_default()
                .push(format!("{}: {name}", involved_people_role(&role_key)));
        } else {
            grouped
                .entry((target.0.to_string(), target.1))
                .or_default()
                .push(name.to_string());
        }
    }
    grouped
        .into_iter()
        .map(|((field, frame), names)| (field, frame, names.join(" / ")))
        .collect()
}

fn contributor_role_target(role: &str) -> Option<(&'static str, &'static str)> {
    match role {
        role if role.contains("composer") || role.contains("composed") => {
            Some(("Composer", "TCOM"))
        }
        role if role.contains("lyric") || role.contains("text writer") || role == "writer" => {
            Some(("Lyricist", "TEXT"))
        }
        role if role.contains("album artist")
            || role.contains("band")
            || role.contains("group") =>
        {
            Some(("Album artist", "TPE2"))
        }
        role if role.contains("engineer") => Some(("Involved people", "TIPL")),
        role if role.contains("producer") => Some(("Involved people", "TIPL")),
        role if role.contains("arranger") => Some(("Involved people", "TIPL")),
        role if role.contains("mixer") => Some(("Involved people", "TIPL")),
        role if role.contains("master") => Some(("Involved people", "TIPL")),
        role if role == "musician"
            || role.contains("lead")
            || role.contains("performer")
            || role.contains("artist") =>
        {
            Some(("Lead performer", "TPE1"))
        }
        role if role.contains("conductor") => Some(("Conductor", "TPE3")),
        role if role.contains("remix") => Some(("Remixer", "TPE4")),
        role if role.contains("original artist") => Some(("Original artist", "TOPE")),
        role if role.contains("original lyric") => Some(("Original lyricist", "TOLY")),
        _ => None,
    }
}

fn involved_people_role(role: &str) -> &'static str {
    if role.contains("master") {
        "mastering engineer"
    } else if role.contains("mix") {
        "mix engineer"
    } else if role.contains("engineer") {
        "engineer"
    } else if role.contains("producer") {
        "producer"
    } else if role.contains("arranger") {
        "arranger"
    } else {
        "involved person"
    }
}

fn instrument_role(role: &str) -> Option<&str> {
    let lower = role.trim().to_ascii_lowercase();
    if let Some(instrument) = lower
        .strip_prefix("performer [")
        .and_then(|value| value.strip_suffix(']'))
    {
        return canonical_instrument_role(instrument);
    }
    if let Some(instrument) = lower.strip_prefix("performer: ") {
        return canonical_instrument_role(instrument);
    }
    if let Some(instrument) = lower.strip_prefix("performer - ") {
        return canonical_instrument_role(instrument);
    }
    canonical_instrument_role(&lower)
}

fn canonical_instrument_role(role: &str) -> Option<&'static str> {
    match role {
        "vocal" | "vocals" | "vocalist" | "singer" => Some("vocals"),
        "guitar" | "guitars" | "guitarist" => Some("guitar"),
        "bass" | "bassist" | "bass guitar" => Some("bass"),
        "drum" | "drums" | "drummer" => Some("drums"),
        "keyboard" | "keyboards" | "keyboardist" => Some("keyboards"),
        "piano" | "pianist" => Some("piano"),
        "banjo" | "banjoist" => Some("banjo"),
        "violin" | "violinist" => Some("violin"),
        "cello" | "cellist" => Some("cello"),
        "saxophone" | "saxophonist" => Some("saxophone"),
        "trumpet" | "trumpeter" => Some("trumpet"),
        "percussion" | "percussionist" => Some("percussion"),
        _ => None,
    }
}

pub fn summarize_value_routes(routes: &[PaymentRoute]) -> Option<String> {
    if routes.is_empty() {
        return None;
    }
    serde_json::to_string(routes).ok()
}

pub fn display_metadata_value(field: &str, value: &str) -> String {
    match field {
        "Contributors" => display_picard_people_list(value),
        "Value Routes" => display_value_routes(value),
        "Transcript" => display_embedded_text_summary(value, "Embedded synced transcript"),
        "Transcript text" => display_embedded_text_summary(value, "Embedded transcript"),
        _ => value.to_string(),
    }
}

pub fn display_picard_people_list(value: &str) -> String {
    let entries = grouped_contributor_entries(value);
    if entries.is_empty() {
        return value.to_string();
    }
    entries
        .into_iter()
        .map(|(name, roles)| {
            if roles.is_empty() {
                name
            } else {
                format!("{name}: {}", roles.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn display_contributor_tree(value: &str) -> Option<String> {
    let entries = grouped_contributor_entries(value);
    if entries.is_empty() {
        return None;
    }
    Some(
        entries
            .into_iter()
            .map(|(name, roles)| {
                if roles.is_empty() {
                    name
                } else {
                    let role_lines = roles
                        .into_iter()
                        .map(|role| format!("  - {role}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("{name}\n{role_lines}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub fn display_value_routes(value: &str) -> String {
    display_pretty_json_tree(value).unwrap_or_else(|| value.to_string())
}

pub fn display_pretty_json_tree(value: &str) -> Option<String> {
    let json = serde_json::from_str::<serde_json::Value>(value).ok()?;
    serde_json::to_string_pretty(&json).ok()
}

fn display_embedded_text_summary(value: &str, label: &str) -> String {
    if value.starts_with("http://") || value.starts_with("https://") || value.starts_with('/') {
        return value.to_string();
    }
    let lines = value.lines().filter(|line| !line.trim().is_empty()).count();
    if lines == 0 {
        label.to_string()
    } else {
        format!("{label} ({lines} line{})", plural(lines))
    }
}

pub fn summarize_contributor_value(value: &str) -> Option<String> {
    let count = grouped_contributor_entries(value).len();
    match count {
        0 => None,
        1 => Some("1 contributor".into()),
        count => Some(format!("{count} contributors")),
    }
}

pub fn expanded_metadata_display_value<'a>(
    field: &str,
    raw_value: &'a str,
    display_value: &'a str,
) -> &'a str {
    match field {
        "Transcript" | "Transcript text" => raw_value,
        _ if display_value.is_empty() => raw_value,
        _ => display_value,
    }
}

pub fn expanded_metadata_display_string(
    field: &str,
    raw_value: &str,
    display_value: &str,
) -> String {
    match field {
        "Contributors" => {
            display_contributor_tree(raw_value).unwrap_or_else(|| display_value.to_string())
        }
        "Transcript" | "Transcript text" => raw_value.to_string(),
        _ if display_value.is_empty() => raw_value.to_string(),
        _ => display_value.to_string(),
    }
}

fn grouped_contributor_entries(value: &str) -> Vec<(String, Vec<String>)> {
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for part in value.split(" / ") {
        let Some((role, name)) = part.split_once(": ") else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let role = role.trim();
        let roles = grouped.entry(name.to_string()).or_default();
        if !role.is_empty() && !roles.iter().any(|existing| existing == role) {
            roles.push(role.to_string());
        }
    }
    grouped.into_iter().collect()
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

pub fn selected_musicbrainz_candidate(
    frame_musicbrainz_selected: usize,
    result: &MusicBrainzLookupResult,
) -> Option<&MusicBrainzCandidate> {
    result
        .lookup
        .candidates
        .get(frame_musicbrainz_selected)
        .or_else(|| result.lookup.candidates.first())
}

pub fn id3_header_title(result: &TagCompareResult) -> String {
    result
        .rows
        .iter()
        .find(|row| row.field == "Title")
        .and_then(|row| row.tag_value.clone())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "Embedded id3".into())
}

// General helpers

pub fn entity_key(entity_type: &str, entity_id: &str) -> String {
    format!("{entity_type}:{entity_id}")
}

pub fn feed_title(feed: &Feed) -> String {
    feed.title
        .clone()
        .or_else(|| feed.name.clone())
        .or_else(|| feed.feed_guid.clone())
        .unwrap_or_else(|| "Untitled".into())
}

pub fn track_title(track: &Track) -> String {
    track
        .title
        .clone()
        .or_else(|| track.name.clone())
        .or_else(|| track.track_guid.clone())
        .unwrap_or_else(|| "Untitled".into())
}

pub fn fmt_dur(secs: i32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

pub fn fmt_ms(ms: i64) -> String {
    fmt_dur((ms / 1000).try_into().unwrap_or(i32::MAX))
}

pub fn join_values(values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.join(" · "))
    }
}

pub fn join_line_values(values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.join("\n"))
    }
}

pub fn fmt_runtime(total_secs: i32) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    if hours > 0 {
        format!("{hours} h {minutes} min")
    } else {
        format!("{minutes} min")
    }
}

pub fn fmt_date(ts: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.format("%b %-d, %Y").to_string())
}

pub fn id3_frame_version(frame_id: &str) -> Id3FrameVersion {
    let frame_id = id3_frame_base(frame_id);
    if frame_id.len() == 3 {
        return Id3FrameVersion::V22;
    }
    if matches!(
        frame_id,
        "ASPI"
            | "EQU2"
            | "RVA2"
            | "SEEK"
            | "SIGN"
            | "TDEN"
            | "TDOR"
            | "TDRC"
            | "TDRL"
            | "TDTG"
            | "TIPL"
            | "TMCL"
            | "TMOO"
            | "TPRO"
            | "TSOA"
            | "TSOP"
            | "TSOT"
            | "TSST"
    ) {
        return Id3FrameVersion::V24Only;
    }
    if matches!(
        frame_id,
        "CRM" | "EQUA" | "IPLS" | "RVAD" | "TDAT" | "TIME" | "TORY" | "TRDA" | "TSIZ" | "TYER"
    ) {
        return Id3FrameVersion::V23Only;
    }
    if ID3V24_FRAME_IDS.contains(&frame_id) {
        Id3FrameVersion::V23V24
    } else {
        Id3FrameVersion::Unknown
    }
}

pub fn id3_frame_hint(field: &str) -> Option<&'static str> {
    if performer_instrument_field(field).is_some() {
        return Some("TMCL");
    }
    match field {
        "Title" => Some("TIT2"),
        "Artist" => Some("TPE1"),
        "Album/Feed" => Some("TALB"),
        "Track #" => Some("TRCK"),
        "Publisher" => Some("TXXX:V4V_PUBLISHER"),
        "RSS feed guid" => Some("TXXX:MusicIndex Feed Guid"),
        "RSS track guid" => Some("TXXX:MusicIndex Track Guid"),
        "Nostr handle" | "RSS feed nostr handle" => Some("TXXX:RSS Nostr Handle"),
        "Label" => Some("TPUB"),
        "Website" | "RSS feed website" => Some("WOAR"),
        "Tempo" => Some("TBPM"),
        "Release date" => Some("TDRC"),
        "Release year" => Some("TYER"),
        "Duration" => Some("TLEN"),
        "Artwork" => Some("APIC"),
        "Description" => Some("COMM:MusicIndex Description"),
        "Transcript" => Some("SYLT:MusicIndex Transcript"),
        "Transcript text" => Some("USLT:MusicIndex Transcript"),
        "Contributors" => Some("TXXX:MusicIndex Contributors"),
        "Composer" => Some("TCOM"),
        "Lyricist" => Some("TEXT"),
        "Lead performer" => Some("TPE1"),
        "Album artist" => Some("TPE2"),
        "Conductor" => Some("TPE3"),
        "Remixer" => Some("TPE4"),
        "Original artist" => Some("TOPE"),
        "Original lyricist" => Some("TOLY"),
        "Involved musicians" => Some("TMCL"),
        "Value Routes" => Some("TXXX:MusicIndex Value Routes"),
        "MusicBrainz recording" => Some("UFID:http://musicbrainz.org"),
        "MusicBrainz release" => Some("TXXX:MusicBrainz Album Id"),
        "MusicBrainz release group" => Some("TXXX:MusicBrainz Release Group Id"),
        "Release country" => Some("TXXX:MusicBrainz Album Release Country"),
        "Release status" => Some("TXXX:MusicBrainz Album Status"),
        "Barcode" => Some("TXXX:BARCODE"),
        "Release type" | "Release secondary types" => Some("TXXX:MusicBrainz Album Type"),
        "Media" => Some("TMED"),
        "Disc #" => Some("TPOS"),
        "Disc subtitle" => Some("TSST"),
        "Total tracks" => Some("TRCK"),
        "ISRC" => Some("TSRC"),
        _ => None,
    }
}

pub fn id3_frame_is_summarized(frame_id: &str) -> bool {
    matches!(frame_id, "TIT2" | "TPE1" | "TALB" | "TRCK")
}

pub fn format_track_slash_total(track: Option<&str>, total: Option<&str>) -> Option<String> {
    match (track, total) {
        (Some(t), Some(tot)) => {
            let position = t.split_once('/').map_or(t, |(head, _)| head);
            Some(format!("{position}/{tot}"))
        }
        (Some(t), None) => Some(t.to_string()),
        (None, Some(tot)) => Some(format!("/{tot}")),
        (None, None) => None,
    }
}

pub fn metadata_drag_value(
    row_id: String,
    field: String,
    frame: String,
    target_existing_value: Option<String>,
    value: String,
    source: MetadataColumn,
) -> MetadataDragValue {
    MetadataDragValue {
        row_id,
        field,
        frame,
        target_existing_value,
        value,
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        aligned_compare_rows, compare_track_rows, contributor_id3_rows, display_contributor_tree,
        display_metadata_value, expanded_metadata_display_string, expanded_metadata_display_value,
        id3_frame_base, musicindex_contributors_id3_value, pending_id3_target_key,
        sanitize_track_context_source_text, source_text_is_placeholder,
        summarize_contributor_value, track_metadata_rows, MetadataGridRow, TagCompareResult,
        TrackContext,
    };
    use std::collections::BTreeSet;

    use crate::api::{
        Contributor, Feed, SourceEntityId, SourceEntityLink, SourceReleaseClaim, Track,
    };
    use crate::audio_tags::AudioTags;
    use crate::track_compare::{ComparisonRow, ComparisonStatus};

    #[test]
    fn id3_target_keys_normalize_descriptor_control_chars() {
        assert_eq!(
            pending_id3_target_key("TXXX: \0MusicIndex Contributors\t"),
            "TXXX:musicindex contributors"
        );
        assert_eq!(id3_frame_base(" TXXX : descriptor "), "TXXX");
    }

    #[test]
    fn contributor_values_display_as_grouped_name_role_lines() {
        let value = "guitarist: Alice / musician: Alice / audio engineer: Bob";
        assert_eq!(
            display_metadata_value("Contributors", value),
            "Alice: guitarist, musician\nBob: audio engineer"
        );
        assert_eq!(
            summarize_contributor_value(value).as_deref(),
            Some("2 contributors")
        );
    }

    #[test]
    fn contributor_tree_displays_people_with_indented_roles() {
        let value = "guitarist: Alice / musician: Alice / audio engineer: Bob";
        assert_eq!(
            display_contributor_tree(value).as_deref(),
            Some("Alice\n  - guitarist\n  - musician\nBob\n  - audio engineer")
        );
        assert_eq!(
            expanded_metadata_display_string("Contributors", value, "2 contributors"),
            "Alice\n  - guitarist\n  - musician\nBob\n  - audio engineer"
        );
    }

    #[test]
    fn contributor_tree_falls_back_for_unstructured_values() {
        assert_eq!(display_contributor_tree("Alice and Bob"), None);
        assert_eq!(
            expanded_metadata_display_string("Contributors", "Alice and Bob", "Alice and Bob"),
            "Alice and Bob"
        );
    }

    #[test]
    fn expanded_transcript_uses_raw_text() {
        assert_eq!(
            expanded_metadata_display_value(
                "Transcript text",
                "line one\nline two",
                "Embedded transcript (2 lines)"
            ),
            "line one\nline two"
        );
    }

    #[test]
    fn source_placeholder_detection_covers_multiline_ellipsis_payloads() {
        assert!(source_text_is_placeholder("..."));
        assert!(source_text_is_placeholder("\u{2026}"));
        assert!(source_text_is_placeholder("...\n...\n..."));
        assert!(source_text_is_placeholder(" . . . \n \u{2026} "));
        assert!(source_text_is_placeholder("<p>...</p><p>...</p>"));
        assert!(source_text_is_placeholder("&hellip;"));
        assert!(source_text_is_placeholder("&#8230;"));
        assert!(source_text_is_placeholder("&nbsp;<br />..."));
        assert!(source_text_is_placeholder("\u{200B}...\u{200B}"));
        assert!(!source_text_is_placeholder(
            "All music by Emily Whitehurst."
        ));
        assert!(!source_text_is_placeholder("... real text"));
        assert!(!source_text_is_placeholder("<p>Real text ...</p>"));
        assert!(!source_text_is_placeholder("<3>"));
    }

    #[test]
    fn sanitize_track_context_source_text_clears_placeholder_display_facts() {
        let mut context = TrackContext {
            track: Track {
                feed_title: Some("...".into()),
                feed_url: Some("\u{2026}".into()),
                title: Some("...\n...\n...".into()),
                name: Some("Real fallback".into()),
                description: Some(" . . . ".into()),
                enclosure_url: Some("...".into()),
                image_url: Some("...".into()),
                source_links: Some(vec![
                    SourceEntityLink {
                        link_type: Some("website".into()),
                        url: Some("...".into()),
                        ..Default::default()
                    },
                    SourceEntityLink {
                        link_type: Some("website".into()),
                        url: Some("https://example.test".into()),
                        ..Default::default()
                    },
                ]),
                source_ids: Some(vec![SourceEntityId {
                    scheme: Some("nostr_npub".into()),
                    value: Some("&hellip;".into()),
                    ..Default::default()
                }]),
                source_release_claims: Some(vec![SourceReleaseClaim {
                    claim_type: Some("description".into()),
                    claim_value: Some("<p>...</p><p>...</p>".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            feed: Some(Feed {
                title: Some("...".into()),
                name: Some("Real feed".into()),
                feed_url: Some("...".into()),
                description: Some("Real feed description".into()),
                source_links: Some(vec![SourceEntityLink {
                    link_type: Some("website".into()),
                    url: Some("\u{2026}".into()),
                    ..Default::default()
                }]),
                source_ids: Some(vec![SourceEntityId {
                    scheme: Some("nostr_npub".into()),
                    value: Some("npub1real".into()),
                    ..Default::default()
                }]),
                source_release_claims: Some(vec![SourceReleaseClaim {
                    claim_type: Some("description".into()),
                    claim_value: Some("Feed source description".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
        };

        sanitize_track_context_source_text(&mut context);

        assert_eq!(context.track.feed_title, None);
        assert_eq!(context.track.feed_url, None);
        assert_eq!(context.track.title, None);
        assert_eq!(context.track.name.as_deref(), Some("Real fallback"));
        assert_eq!(context.track.description, None);
        assert_eq!(context.track.enclosure_url, None);
        assert_eq!(context.track.image_url, None);
        assert_eq!(
            context
                .track
                .source_links
                .as_deref()
                .and_then(|links| links.first())
                .and_then(|link| link.url.as_deref()),
            Some("https://example.test")
        );
        assert_eq!(
            context
                .track
                .source_ids
                .as_deref()
                .and_then(|ids| ids.first())
                .and_then(|id| id.value.as_deref()),
            None
        );
        assert_eq!(
            context
                .track
                .source_release_claims
                .as_deref()
                .and_then(|claims| claims.first())
                .and_then(|claim| claim.claim_value.as_deref()),
            None
        );
        let feed = context.feed.as_ref().expect("feed remains present");
        assert_eq!(feed.title, None);
        assert_eq!(feed.name.as_deref(), Some("Real feed"));
        assert_eq!(feed.feed_url, None);
        assert_eq!(feed.description.as_deref(), Some("Real feed description"));
        assert!(feed.source_links.as_deref().is_none_or(<[_]>::is_empty));
        assert_eq!(
            feed.source_ids
                .as_deref()
                .and_then(|ids| ids.first())
                .and_then(|id| id.value.as_deref()),
            Some("npub1real")
        );
        assert_eq!(
            feed.source_release_claims
                .as_deref()
                .and_then(|claims| claims.first())
                .and_then(|claim| claim.claim_value.as_deref()),
            Some("Feed source description")
        );
    }

    #[test]
    fn compare_track_rows_drop_placeholder_source_values() {
        let track = Track {
            title: Some("...\n...\n...".into()),
            track_artist: Some("\u{2026}".into()),
            feed_title: Some(" . . . ".into()),
            publisher_text: Some("...".into()),
            ..Default::default()
        };
        let tags = AudioTags {
            title: Some("Embedded title".into()),
            ..Default::default()
        };

        let rows = compare_track_rows(&track, None, &tags);

        let title_row = rows
            .iter()
            .find(|row| row.field == "Title")
            .expect("embedded title keeps row visible");
        assert_eq!(title_row.source_value, None);
        assert_eq!(title_row.status, ComparisonStatus::MissingSource);
        assert!(
            rows.iter().all(|row| row
                .source_value
                .as_deref()
                .is_none_or(|value| !source_text_is_placeholder(value))),
            "compare rows must never carry placeholder source facts"
        );
    }

    #[test]
    fn aligned_compare_rows_refills_placeholder_result_sources_from_context() {
        let result = TagCompareResult {
            path: "track.flac".into(),
            rows: vec![
                ComparisonRow {
                    field: "Title",
                    source_value: Some("...".into()),
                    tag_value: Some("Embedded title".into()),
                    status: ComparisonStatus::Different,
                },
                ComparisonRow {
                    field: "Artist",
                    source_value: Some("\u{2026}".into()),
                    tag_value: None,
                    status: ComparisonStatus::MissingTag,
                },
                ComparisonRow {
                    field: "Album/Feed",
                    source_value: Some("...\n...\n...".into()),
                    tag_value: None,
                    status: ComparisonStatus::MissingTag,
                },
                ComparisonRow {
                    field: "Track #",
                    source_value: Some("...".into()),
                    tag_value: Some("3".into()),
                    status: ComparisonStatus::Different,
                },
            ],
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            id3_fields: Vec::new(),
            total_tracks: Some("18".into()),
            format: None,
        };
        let track_context = TrackContext {
            track: Track {
                title: Some("The Platform".into()),
                track_artist: Some("HeyCitizen".into()),
                feed_title: Some("The Heycitizen Experience".into()),
                track_number: Some(3),
                duration_secs: Some(294),
                description: Some("HeyCitizen makes sounds.".into()),
                ..Default::default()
            },
            feed: Some(Feed {
                episode_count: Some(18),
                ..Default::default()
            }),
        };

        let rows = aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());

        assert_eq!(
            data_row(&rows, "Title").and_then(|row| row.rss_value.as_deref()),
            Some("The Platform")
        );
        assert_eq!(
            data_row(&rows, "Artist").and_then(|row| row.rss_value.as_deref()),
            Some("HeyCitizen")
        );
        assert_eq!(
            data_row(&rows, "Album/Feed").and_then(|row| row.rss_value.as_deref()),
            Some("The Heycitizen Experience")
        );
        assert_eq!(
            data_row(&rows, "Track #").and_then(|row| row.rss_value.as_deref()),
            Some("3/18")
        );
        assert!(
            rows.iter()
                .filter_map(|row| match row {
                    MetadataGridRow::Data(row) => row.rss_value.as_deref(),
                    MetadataGridRow::Group(_) => None,
                })
                .all(|value| !source_text_is_placeholder(value)),
            "aligned metadata rows must not display placeholder RSS values"
        );
    }

    #[test]
    fn track_metadata_rows_drop_markup_placeholder_source_values() {
        let track_context = TrackContext {
            track: Track {
                title: Some("<p>...</p><p>...</p>".into()),
                name: Some("Real title".into()),
                track_artist: Some("&hellip;".into()),
                release_artist: Some("Real artist".into()),
                feed_title: Some("&nbsp;<br />...".into()),
                publisher_text: Some("&#8230;".into()),
                description: Some("<p>...</p>".into()),
                ..Default::default()
            },
            feed: Some(Feed {
                title: Some("Real feed".into()),
                description: Some("Real feed description".into()),
                ..Default::default()
            }),
        };

        let rows = track_metadata_rows(&track_context, None, false);

        assert_eq!(
            data_row(&rows, "Title").and_then(|row| row.rss_value.as_deref()),
            Some("Real title")
        );
        assert_eq!(
            data_row(&rows, "Artist").and_then(|row| row.rss_value.as_deref()),
            None
        );
        assert_eq!(
            data_row(&rows, "Album artist").and_then(|row| row.rss_value.as_deref()),
            Some("Real artist")
        );
        assert_eq!(
            data_row(&rows, "Album/Feed").and_then(|row| row.rss_value.as_deref()),
            Some("Real feed")
        );
        assert_eq!(
            data_row(&rows, "Publisher").and_then(|row| row.rss_value.as_deref()),
            None
        );
        assert_eq!(
            data_row(&rows, "Description").and_then(|row| row.rss_value.as_deref()),
            Some("Real feed description")
        );
        assert!(
            rows.iter()
                .filter_map(|row| match row {
                    MetadataGridRow::Data(row) => row.rss_value.as_deref(),
                    MetadataGridRow::Group(_) => None,
                })
                .all(|value| !source_text_is_placeholder(value)),
            "track metadata rows must not display markup/entity placeholder source facts"
        );
    }

    fn data_row<'a>(
        rows: &'a [MetadataGridRow],
        field: &str,
    ) -> Option<&'a super::AlignedCompareRow> {
        rows.iter().find_map(|row| match row {
            MetadataGridRow::Data(row) if row.field == field => Some(row),
            MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
        })
    }

    #[test]
    fn sanitize_track_context_strips_placeholder_contributor_names() {
        let mut context = TrackContext {
            track: Track {
                source_contributors: Some(vec![
                    Contributor {
                        name: Some("...".into()),
                        role: Some("composer".into()),
                        ..Contributor::default()
                    },
                    Contributor {
                        name: Some("Alice".into()),
                        role: Some("\u{2026}".into()),
                        ..Contributor::default()
                    },
                    Contributor {
                        name: Some("Bob".into()),
                        role: Some("guitar".into()),
                        ..Contributor::default()
                    },
                ]),
                ..Default::default()
            },
            feed: Some(Feed {
                source_contributors: Some(vec![Contributor {
                    name: Some("&hellip;".into()),
                    role: Some("producer".into()),
                    ..Contributor::default()
                }]),
                ..Default::default()
            }),
        };

        sanitize_track_context_source_text(&mut context);

        let track_contributors = context
            .track
            .source_contributors
            .as_deref()
            .unwrap_or_default();
        assert_eq!(track_contributors.len(), 2);
        assert_eq!(track_contributors[0].name.as_deref(), Some("Alice"));
        assert_eq!(track_contributors[0].role, None);
        assert_eq!(track_contributors[1].name.as_deref(), Some("Bob"));
        assert_eq!(track_contributors[1].role.as_deref(), Some("guitar"));

        let feed_contributors = context
            .feed
            .as_ref()
            .and_then(|feed| feed.source_contributors.as_deref())
            .unwrap_or_default();
        assert!(
            feed_contributors.is_empty(),
            "placeholder-only feed contributor must be dropped at the merge boundary"
        );
    }

    #[test]
    fn contributor_id3_rows_skip_placeholder_names_and_roles() {
        let contributors = vec![
            Contributor {
                name: Some("...".into()),
                role: Some("composer".into()),
                ..Contributor::default()
            },
            Contributor {
                name: Some("Alice".into()),
                role: Some("<p>...</p>".into()),
                ..Contributor::default()
            },
            Contributor {
                name: Some("Bob".into()),
                role: Some("guitar".into()),
                ..Contributor::default()
            },
        ];

        let rows = contributor_id3_rows(&contributors);
        for (_, _, value) in &rows {
            assert!(
                !source_text_is_placeholder(value),
                "contributor row value must not be a placeholder: {value:?}"
            );
            assert!(
                !value.contains("..."),
                "contributor row must not surface raw ellipsis text: {value:?}"
            );
        }
        assert!(
            rows.iter()
                .any(|(field, _, value)| field == "Performer [guitar]" && value == "Bob"),
            "real contributor must survive placeholder filtering"
        );

        let summary = musicindex_contributors_id3_value(&contributors).unwrap_or_default();
        assert!(
            !source_text_is_placeholder(&summary),
            "contributor summary must not be a placeholder: {summary:?}"
        );
        assert!(
            !summary.contains("..."),
            "contributor summary must not surface raw ellipsis text: {summary:?}"
        );
        assert!(summary.contains("Alice"));
        assert!(summary.contains("Bob"));
    }
}
