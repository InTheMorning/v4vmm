use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use id3::frame::{
    Comment, ExtendedLink, ExtendedText, InvolvedPeopleList, InvolvedPeopleListItem, Lyrics,
    Picture, PictureType, SynchronisedLyrics, SynchronisedLyricsType, TimestampFormat,
    UniqueFileIdentifier,
};
use id3::{no_tag_ok, Content, Frame, Tag, TagLike, Version};

const WRITABLE_TEXT_FRAMES: &[&str] = &[
    "TALB", "TBPM", "TCOM", "TCON", "TCOP", "TDEN", "TDLY", "TDOR", "TDRC", "TDRL", "TDTG", "TENC",
    "TEXT", "TFLT", "TIT1", "TIT2", "TIT3", "TKEY", "TLAN", "TLEN", "TMED", "TMOO", "TOAL", "TOFN",
    "TOLY", "TOPE", "TOWN", "TPE1", "TPE2", "TPE3", "TPE4", "TPOS", "TPRO", "TPUB", "TRCK", "TRSN",
    "TRSO", "TSOA", "TSOP", "TSOT", "TSRC", "TSSE", "TSST", "TXXX", "TYER",
];
const WRITABLE_URL_FRAMES: &[&str] = &[
    "WCOM", "WCOP", "WOAF", "WOAR", "WOAS", "WORS", "WPAY", "WPUB", "WXXX",
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<String>,
    pub total_tracks: Option<String>,
    pub date: Option<String>,
    pub custom: BTreeMap<String, String>,
    pub artwork: Option<EmbeddedArtwork>,
    pub fields: Vec<Id3Field>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddedArtwork {
    pub mime_type: String,
    pub picture_type: String,
    pub description: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Id3Field {
    pub frame_id: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Id3v24Edit {
    pub frame_label: String,
    pub value: String,
}

pub fn read_audio_tags(path: &Path) -> Result<AudioTags> {
    read_mp3_tags(path)
}

pub fn write_id3v24_edits(path: &Path, edits: &[Id3v24Edit]) -> Result<usize> {
    if edits.is_empty() {
        return Ok(0);
    }

    let mut tag = no_tag_ok(Tag::read_from_path(path))
        .with_context(|| format!("read embedded MP3 tags from {}", path.display()))?
        .unwrap_or_default();
    let mut applied = 0;

    for edit in edits {
        let frame = id3v24_edit_frame(edit)?;
        tag.add_frame(frame);
        applied += 1;
    }

    tag.write_to_path(path, Version::Id3v24)
        .with_context(|| format!("write ID3v2.4 tags to {}", path.display()))?;
    Ok(applied)
}

fn read_mp3_tags(path: &Path) -> Result<AudioTags> {
    let tag = no_tag_ok(Tag::read_from_path(path))
        .with_context(|| format!("read embedded MP3 tags from {}", path.display()))?
        .unwrap_or_default();

    Ok(audio_tags_from_id3(&tag))
}

fn audio_tags_from_id3(tag: &Tag) -> AudioTags {
    AudioTags {
        title: tag.title().map(ToOwned::to_owned),
        artist: tag.artist().map(ToOwned::to_owned),
        album: tag.album().map(ToOwned::to_owned),
        track_number: tag
            .track()
            .map(|number| number.to_string())
            .or_else(|| first_text_frame(tag, "TRCK")),
        total_tracks: tag
            .total_tracks()
            .map(|number| number.to_string())
            .or_else(|| {
                first_text_frame(tag, "TRCK")
                    .and_then(|trck| trck.split('/').nth(1).map(str::to_string))
            }),
        date: tag
            .year()
            .map(|year| year.to_string())
            .or_else(|| first_text_frame(tag, "TDRC"))
            .or_else(|| first_text_frame(tag, "TYER")),
        custom: read_txxx_map(tag),
        artwork: embedded_artwork(tag),
        fields: id3_fields(tag),
    }
}

fn first_text_frame(tag: &Tag, id: &str) -> Option<String> {
    tag.frames().find_map(|frame| {
        if frame.id() != id {
            return None;
        }

        match frame.content() {
            Content::Text(text) => Some(text.to_string()),
            Content::ExtendedText(ext) => Some(ext.value.to_string()),
            _ => None,
        }
    })
}

fn read_txxx_map(tag: &Tag) -> BTreeMap<String, String> {
    tag.frames()
        .filter_map(|frame| {
            if frame.id() != "TXXX" {
                return None;
            }

            match frame.content() {
                Content::ExtendedText(ext) => {
                    Some((ext.description.to_string(), ext.value.to_string()))
                }
                _ => None,
            }
        })
        .collect()
}

fn embedded_artwork(tag: &Tag) -> Option<EmbeddedArtwork> {
    tag.pictures().next().map(|picture| EmbeddedArtwork {
        mime_type: picture.mime_type.clone(),
        picture_type: picture.picture_type.to_string(),
        description: picture.description.clone(),
        data: picture.data.clone(),
    })
}

fn id3_fields(tag: &Tag) -> Vec<Id3Field> {
    tag.frames().map(id3_field).collect()
}

fn id3_field(frame: &Frame) -> Id3Field {
    match frame.content() {
        Content::ExtendedText(ext) => Id3Field {
            frame_id: descriptor_frame_label("TXXX", &ext.description),
            value: ext.value.clone(),
        },
        Content::ExtendedLink(ext) => Id3Field {
            frame_id: descriptor_frame_label("WXXX", &ext.description),
            value: ext.link.clone(),
        },
        Content::Comment(comment) => Id3Field {
            frame_id: descriptor_frame_label("COMM", &comment.description),
            value: comment.text.clone(),
        },
        Content::Lyrics(lyrics) => Id3Field {
            frame_id: descriptor_frame_label("USLT", &lyrics.description),
            value: lyrics.text.clone(),
        },
        Content::SynchronisedLyrics(lyrics) => Id3Field {
            frame_id: descriptor_frame_label("SYLT", &lyrics.description),
            value: synchronised_lyrics_display_value(lyrics),
        },
        Content::UniqueFileIdentifier(ufid) => Id3Field {
            frame_id: descriptor_frame_label("UFID", &ufid.owner_identifier),
            value: String::from_utf8(ufid.identifier.clone())
                .unwrap_or_else(|_| format!("{:x?}", &ufid.identifier)),
        },
        Content::InvolvedPeopleList(list) => Id3Field {
            frame_id: frame.id().to_string(),
            value: format_involved_people_list(list),
        },
        _ => Id3Field {
            frame_id: frame.id().to_string(),
            value: frame.content().to_string(),
        },
    }
}

fn format_involved_people_list(list: &InvolvedPeopleList) -> String {
    list.items
        .iter()
        .map(|item| format!("{}: {}", item.involvement.trim(), item.involvee.trim()))
        .collect::<Vec<_>>()
        .join(" / ")
}

fn descriptor_frame_label(frame_id: &str, descriptor: &str) -> String {
    match normalize_frame_descriptor(descriptor) {
        Some(descriptor) => format!("{frame_id}:{descriptor}"),
        None => frame_id.to_string(),
    }
}

fn synchronised_lyrics_display_value(lyrics: &SynchronisedLyrics) -> String {
    if lyrics.content.is_empty() {
        return lyrics.content_type.to_string();
    }
    let mut bytes = Vec::new();
    if lyrics.fmt_table(&mut bytes).is_ok() {
        String::from_utf8(bytes).unwrap_or_else(|_| lyrics.content_type.to_string())
    } else {
        lyrics.content_type.to_string()
    }
}

fn id3v24_edit_frame(edit: &Id3v24Edit) -> Result<Frame> {
    let (frame_id, descriptor) = split_frame_label(&edit.frame_label);
    let value = edit.value.trim();
    if frame_id.is_empty() {
        return Err(anyhow!("missing ID3 frame id"));
    }
    if value.is_empty() {
        return Err(anyhow!("missing ID3 value for {frame_id}"));
    }

    let frame_id = frame_id.as_str();
    if !id3v24_edit_frame_is_writable(frame_id) {
        return Err(anyhow!("unsupported ID3v2.4 edit frame {frame_id}"));
    }

    match frame_id {
        "TXXX" => Ok(Frame::with_content(
            "TXXX",
            Content::ExtendedText(ExtendedText {
                description: required_frame_descriptor(frame_id, descriptor)?,
                value: value.to_string(),
            }),
        )),
        "WXXX" => Ok(Frame::with_content(
            "WXXX",
            Content::ExtendedLink(ExtendedLink {
                description: required_frame_descriptor(frame_id, descriptor)?,
                link: value.to_string(),
            }),
        )),
        "UFID" => {
            let owner_identifier = required_frame_descriptor(frame_id, descriptor)?;
            Ok(Frame::with_content(
                "UFID",
                Content::UniqueFileIdentifier(UniqueFileIdentifier {
                    owner_identifier,
                    identifier: value.as_bytes().to_vec(),
                }),
            ))
        }
        "COMM" => Ok(Frame::with_content(
            "COMM",
            Content::Comment(Comment {
                lang: "eng".into(),
                description: required_frame_descriptor(frame_id, descriptor)?,
                text: value.to_string(),
            }),
        )),
        "USLT" => uslt_frame_from_reference(value, descriptor),
        "SYLT" => sylt_frame_from_reference(value, descriptor),
        "APIC" => apic_frame_from_reference(value),
        "TIPL" | "TMCL" => Ok(Frame::with_content(
            frame_id,
            Content::InvolvedPeopleList(parse_involved_people_list(value)),
        )),
        "TIT2" => Ok(Frame::text("TIT2", sanitize_title_text(value))),
        id if id.starts_with('T') => Ok(Frame::text(id, value)),
        id if id.starts_with('W') => Ok(Frame::with_content(id, Content::Link(value.to_string()))),
        _ => unreachable!("frame writability was checked before construction"),
    }
}

pub fn id3v24_edit_label_is_writable(frame_label: &str) -> bool {
    let (frame_id, descriptor) = split_frame_label(frame_label);
    if !id3v24_edit_frame_is_writable(&frame_id) {
        return false;
    }
    !matches!(
        frame_id.as_str(),
        "TXXX" | "WXXX" | "UFID" | "COMM" | "USLT" | "SYLT"
    ) || descriptor.is_some()
}

fn id3v24_edit_frame_is_writable(frame_id: &str) -> bool {
    WRITABLE_TEXT_FRAMES.contains(&frame_id)
        || WRITABLE_URL_FRAMES.contains(&frame_id)
        || frame_id == "APIC"
        || matches!(frame_id, "COMM" | "USLT" | "SYLT")
        || frame_id == "UFID"
        || matches!(frame_id, "TIPL" | "TMCL")
}

fn required_frame_descriptor(frame_id: &str, descriptor: Option<String>) -> Result<String> {
    descriptor.ok_or_else(|| anyhow!("{frame_id} edits require a descriptor"))
}

/// Parse a `"role: name / role: name"` display string back into an [`InvolvedPeopleList`].
fn parse_involved_people_list(value: &str) -> InvolvedPeopleList {
    let items = value
        .split(" / ")
        .filter_map(|entry| {
            let (involvement, involvee) = entry.split_once(": ")?;
            Some(InvolvedPeopleListItem {
                involvement: involvement.trim().to_string(),
                involvee: involvee.trim().to_string(),
            })
        })
        .collect();
    InvolvedPeopleList { items }
}

fn sanitize_title_text(value: &str) -> String {
    value
        .trim_start()
        .strip_prefix("- ")
        .map(str::trim_start)
        .unwrap_or(value)
        .to_string()
}

fn split_frame_label(label: &str) -> (String, Option<String>) {
    let Some((frame_id, descriptor)) = label.split_once(':') else {
        return (label.trim().to_ascii_uppercase(), None);
    };
    (
        frame_id.trim().to_ascii_uppercase(),
        normalize_frame_descriptor(descriptor),
    )
}

fn normalize_frame_descriptor(descriptor: &str) -> Option<String> {
    let descriptor = descriptor.replace('\0', " ");
    let descriptor = descriptor.split_whitespace().collect::<Vec<_>>().join(" ");
    (!descriptor.is_empty()).then(|| descriptor.to_string())
}

fn apic_frame_from_reference(reference: &str) -> Result<Frame> {
    let (mime_type, data) = read_picture_reference(reference)?;
    Ok(Frame::with_content(
        "APIC",
        Content::Picture(Picture {
            mime_type,
            picture_type: PictureType::CoverFront,
            description: "front cover".into(),
            data,
        }),
    ))
}

fn uslt_frame_from_reference(reference: &str, descriptor: Option<String>) -> Result<Frame> {
    let description = required_frame_descriptor("USLT", descriptor)?;
    let transcript = parse_transcript_reference(reference)?;
    Ok(Frame::with_content(
        "USLT",
        Content::Lyrics(Lyrics {
            lang: "eng".into(),
            description,
            text: transcript.plain_text,
        }),
    ))
}

fn sylt_frame_from_reference(reference: &str, descriptor: Option<String>) -> Result<Frame> {
    let description = required_frame_descriptor("SYLT", descriptor)?;
    let transcript = parse_transcript_reference(reference)?;
    let content = if transcript.timed_lines.is_empty() {
        vec![(0, transcript.plain_text)]
    } else {
        transcript.timed_lines
    };
    Ok(Frame::with_content(
        "SYLT",
        Content::SynchronisedLyrics(SynchronisedLyrics {
            lang: "eng".into(),
            timestamp_format: TimestampFormat::Ms,
            content_type: SynchronisedLyricsType::Transcription,
            description,
            content,
        }),
    ))
}

#[derive(Debug, Default)]
struct ParsedTranscript {
    timed_lines: Vec<(u32, String)>,
    plain_text: String,
}

fn parse_transcript_reference(reference: &str) -> Result<ParsedTranscript> {
    let text = read_text_reference(reference)?;
    let mut parsed = parse_srt_or_vtt_transcript(&text);
    if parsed.timed_lines.is_empty() {
        parsed = parse_lrc_transcript(&text);
    }
    if parsed.timed_lines.is_empty() {
        parsed = parse_microdvd_sub_transcript(&text);
    }
    if parsed.plain_text.trim().is_empty() {
        parsed.plain_text = strip_timecode_data(&text);
    }
    if parsed.plain_text.trim().is_empty() {
        return Err(anyhow!("transcript is empty"));
    }
    Ok(parsed)
}

fn read_text_reference(reference: &str) -> Result<String> {
    if reference.starts_with("http://") || reference.starts_with("https://") {
        return reqwest::blocking::get(reference)
            .with_context(|| format!("download transcript {reference}"))?
            .error_for_status()
            .with_context(|| format!("download transcript {reference}"))?
            .text()
            .with_context(|| format!("read transcript {reference}"));
    }
    fs::read_to_string(reference).with_context(|| format!("read transcript {reference}"))
}

fn parse_srt_or_vtt_transcript(text: &str) -> ParsedTranscript {
    let mut timed_lines = Vec::new();
    for block in text.replace("\r\n", "\n").split("\n\n") {
        let lines = block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && *line != "WEBVTT")
            .collect::<Vec<_>>();
        let Some(timestamp_index) = lines.iter().position(|line| line.contains("-->")) else {
            continue;
        };
        let Some(start) = lines[timestamp_index]
            .split("-->")
            .next()
            .and_then(parse_timecode_ms)
        else {
            continue;
        };
        let text = lines
            .iter()
            .skip(timestamp_index + 1)
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            timed_lines.push((start, text));
        }
    }
    transcript_from_timed_lines(timed_lines)
}

fn parse_lrc_transcript(text: &str) -> ParsedTranscript {
    let mut timed_lines = Vec::new();
    for line in text.lines() {
        let mut rest = line.trim();
        let mut starts = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some((stamp, remaining)) = stripped.split_once(']') else {
                break;
            };
            let Some(ms) = parse_lrc_timecode_ms(stamp) else {
                break;
            };
            starts.push(ms);
            rest = remaining.trim_start();
        }
        if rest.is_empty() {
            continue;
        }
        timed_lines.extend(starts.into_iter().map(|start| (start, rest.to_string())));
    }
    transcript_from_timed_lines(timed_lines)
}

fn parse_microdvd_sub_transcript(text: &str) -> ParsedTranscript {
    let mut timed_lines = Vec::new();
    for line in text.lines().map(str::trim) {
        let Some(after_open) = line.strip_prefix('{') else {
            continue;
        };
        let Some((start_frame, rest)) = after_open.split_once('}') else {
            continue;
        };
        let Some(rest) = rest.strip_prefix('{') else {
            continue;
        };
        let Some((_end_frame, text)) = rest.split_once('}') else {
            continue;
        };
        let Ok(start_frame) = start_frame.parse::<u32>() else {
            continue;
        };
        let text = text.replace('|', " ").trim().to_string();
        if !text.is_empty() {
            timed_lines.push((start_frame.saturating_mul(40), text));
        }
    }
    transcript_from_timed_lines(timed_lines)
}

fn transcript_from_timed_lines(timed_lines: Vec<(u32, String)>) -> ParsedTranscript {
    let plain_text = timed_lines
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    ParsedTranscript {
        timed_lines,
        plain_text,
    }
}

fn parse_timecode_ms(value: &str) -> Option<u32> {
    let time = value
        .split_whitespace()
        .next()
        .unwrap_or("")
        .replace(',', ".");
    let mut parts = time.split(':').collect::<Vec<_>>();
    if parts.len() == 2 {
        parts.insert(0, "0");
    }
    let [hours, minutes, seconds] = parts.as_slice() else {
        return None;
    };
    let hours = hours.parse::<u32>().ok()?;
    let minutes = minutes.parse::<u32>().ok()?;
    let (seconds, millis) = seconds
        .split_once('.')
        .map_or((*seconds, "0"), |(seconds, millis)| (seconds, millis));
    let seconds = seconds.parse::<u32>().ok()?;
    let millis = parse_millis(millis)?;
    hours
        .checked_mul(3_600_000)?
        .checked_add(minutes.checked_mul(60_000)?)?
        .checked_add(seconds.checked_mul(1_000)?)?
        .checked_add(millis)
}

fn parse_lrc_timecode_ms(value: &str) -> Option<u32> {
    if !value.chars().next()?.is_ascii_digit() {
        return None;
    }
    parse_timecode_ms(&format!("0:{value}"))
}

fn parse_millis(value: &str) -> Option<u32> {
    let digits = value
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return Some(0);
    }
    let mut padded = digits;
    while padded.len() < 3 {
        padded.push('0');
    }
    padded.get(..3)?.parse::<u32>().ok()
}

fn strip_timecode_data(text: &str) -> String {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed == "WEBVTT"
                || trimmed.chars().all(|ch| ch.is_ascii_digit())
                || trimmed.contains("-->")
            {
                return None;
            }
            if trimmed.starts_with('[') && trimmed.contains(']') {
                return trimmed
                    .rsplit(']')
                    .next()
                    .map(str::trim)
                    .filter(|text| !text.is_empty());
            }
            if trimmed.starts_with('{') && trimmed.contains('}') {
                return trimmed
                    .rsplit('}')
                    .next()
                    .map(str::trim)
                    .filter(|text| !text.is_empty());
            }
            Some(trimmed)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_picture_reference(reference: &str) -> Result<(String, Vec<u8>)> {
    if reference.starts_with("http://") || reference.starts_with("https://") {
        let response = reqwest::blocking::get(reference)
            .with_context(|| format!("download APIC image {reference}"))?
            .error_for_status()
            .with_context(|| format!("download APIC image {reference}"))?;
        let mime_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(image_mime_type)
            .ok_or_else(|| anyhow!("APIC image response missing image content type"))?;
        let data = response
            .bytes()
            .with_context(|| format!("read APIC image {reference}"))?
            .to_vec();
        if data.is_empty() {
            return Err(anyhow!("APIC image is empty"));
        }
        return Ok((mime_type, data));
    }

    let path = Path::new(reference);
    let data = fs::read(path).with_context(|| format!("read APIC image {}", path.display()))?;
    if data.is_empty() {
        return Err(anyhow!("APIC image is empty"));
    }
    let mime_type = image_mime_type_for_path(path)
        .ok_or_else(|| anyhow!("unsupported APIC image type for {}", path.display()))?;
    Ok((mime_type, data))
}

fn image_mime_type(value: &str) -> Option<String> {
    let mime_type = value.split(';').next()?.trim().to_ascii_lowercase();
    mime_type.starts_with("image/").then_some(mime_type)
}

fn image_mime_type_for_path(path: &Path) -> Option<String> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg".into()),
        "png" => Some("image/png".into()),
        "gif" => Some("image/gif".into()),
        "webp" => Some("image/webp".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use id3::frame::{ExtendedText, Picture, PictureType};
    use id3::{Frame, Tag, TagLike};

    use super::{
        audio_tags_from_id3, id3v24_edit_label_is_writable, normalize_frame_descriptor,
        read_audio_tags, write_id3v24_edits, AudioTags, EmbeddedArtwork, Id3Field, Id3v24Edit,
    };

    #[test]
    fn maps_id3_frames_to_audio_tags() {
        let mut tag = Tag::new();
        tag.set_title("Song Title");
        tag.set_artist("Track Artist");
        tag.set_album("Feed Title");
        tag.set_track(7);
        tag.set_total_tracks(10);
        tag.set_year(2026);
        tag.add_frame(Frame::with_content(
            "TXXX",
            id3::Content::ExtendedText(ExtendedText {
                description: "V4V_PUBLISHER".into(),
                value: "Wavlake".into(),
            }),
        ));
        tag.add_frame(Frame::with_content(
            "APIC",
            id3::Content::Picture(Picture {
                mime_type: "image/png".into(),
                picture_type: PictureType::CoverFront,
                description: "front".into(),
                data: vec![1, 2, 3],
            }),
        ));

        assert_eq!(
            audio_tags_from_id3(&tag),
            AudioTags {
                title: Some("Song Title".into()),
                artist: Some("Track Artist".into()),
                album: Some("Feed Title".into()),
                track_number: Some("7".into()),
                total_tracks: Some("10".into()),
                date: Some("2026".into()),
                custom: BTreeMap::from([("V4V_PUBLISHER".into(), "Wavlake".into())]),
                artwork: Some(EmbeddedArtwork {
                    mime_type: "image/png".into(),
                    picture_type: "Front cover".into(),
                    description: "front".into(),
                    data: vec![1, 2, 3],
                }),
                fields: vec![
                    Id3Field {
                        frame_id: "TIT2".into(),
                        value: "Song Title".into(),
                    },
                    Id3Field {
                        frame_id: "TPE1".into(),
                        value: "Track Artist".into(),
                    },
                    Id3Field {
                        frame_id: "TALB".into(),
                        value: "Feed Title".into(),
                    },
                    Id3Field {
                        frame_id: "TRCK".into(),
                        value: "7/10".into(),
                    },
                    Id3Field {
                        frame_id: "TYER".into(),
                        value: "2026".into(),
                    },
                    Id3Field {
                        frame_id: "TXXX:V4V_PUBLISHER".into(),
                        value: "Wavlake".into(),
                    },
                    Id3Field {
                        frame_id: "APIC".into(),
                        value: "front: Front cover (image/png, 3 bytes)".into(),
                    },
                ],
            }
        );
    }

    #[test]
    fn missing_id3_tag_returns_blank_tags() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"not really an mp3").expect("write file");

        assert_eq!(
            read_audio_tags(temp.path()).expect("read blank tags"),
            AudioTags::default()
        );
    }

    #[test]
    fn writes_staged_id3v24_text_extended_url_and_ufid_frames() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"not really an mp3").expect("write file");
        let image = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .expect("temp image file");
        fs::write(image.path(), [1, 2, 3]).expect("write image file");
        let transcript = tempfile::Builder::new()
            .suffix(".srt")
            .tempfile()
            .expect("temp transcript file");
        fs::write(
            transcript.path(),
            "1\n00:00:01,250 --> 00:00:03,000\nHello world\n",
        )
        .expect("write transcript file");

        let edits = [
            Id3v24Edit {
                frame_label: "TIT2".into(),
                value: " - Song".into(),
            },
            Id3v24Edit {
                frame_label: "TXXX:MusicIndex Contributors".into(),
                value: "Alice".into(),
            },
            Id3v24Edit {
                frame_label: "WOAR".into(),
                value: "https://example.test".into(),
            },
            Id3v24Edit {
                frame_label: "WOAR".into(),
                value: "https://musicbrainz.example.test".into(),
            },
            Id3v24Edit {
                frame_label: "UFID:http://musicbrainz.org".into(),
                value: "recording-id".into(),
            },
            Id3v24Edit {
                frame_label: "APIC".into(),
                value: image.path().display().to_string(),
            },
            Id3v24Edit {
                frame_label: "COMM:MusicIndex Description".into(),
                value: "RSS description".into(),
            },
            Id3v24Edit {
                frame_label: "USLT:MusicIndex Transcript".into(),
                value: transcript.path().display().to_string(),
            },
            Id3v24Edit {
                frame_label: "SYLT:MusicIndex Transcript".into(),
                value: transcript.path().display().to_string(),
            },
            Id3v24Edit {
                frame_label: "TMCL".into(),
                value: "guitar: Alice / vocals: Bob".into(),
            },
        ];

        assert_eq!(
            write_id3v24_edits(temp.path(), &edits).expect("write ID3v2.4 edits"),
            10
        );

        let tag = Tag::read_from_path(temp.path()).expect("read written ID3 tag");
        assert_eq!(tag.title(), Some("Song"));
        assert!(tag.frames().any(|frame| {
            frame.id() == "TXXX" && frame.content().to_string() == "MusicIndex Contributors: Alice"
        }));
        assert!(tag
            .frames()
            .any(|frame| frame.id() == "WOAR"
                && frame.content().to_string() == "https://example.test"));
        let woar_values = tag
            .frames()
            .filter(|frame| frame.id() == "WOAR")
            .map(|frame| frame.content().to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            woar_values,
            vec![
                "https://example.test".to_string(),
                "https://musicbrainz.example.test".to_string()
            ]
        );
        assert!(tag.frames().any(|frame| {
            frame.id() == "UFID"
                && frame.content().to_string() == "http://musicbrainz.org: recording-id"
        }));
        assert!(tag.frames().any(|frame| {
            frame.id() == "APIC"
                && frame.content().to_string() == "front cover: Front cover (image/png, 3 bytes)"
        }));
        assert!(tag.frames().any(|frame| {
            frame.id() == "COMM"
                && frame.content().to_string() == "MusicIndex Description: RSS description"
        }));
        assert!(tag.frames().any(|frame| {
            frame.id() == "USLT"
                && frame.content().to_string() == "MusicIndex Transcript: Hello world"
        }));
        assert!(tag.frames().any(|frame| {
            matches!(
                frame.content(),
                id3::Content::SynchronisedLyrics(lyrics)
                    if frame.id() == "SYLT"
                        && lyrics.content == vec![(1250, "Hello world".to_string())]
            )
        }));
        assert!(tag.frames().any(|frame| {
            matches!(
                frame.content(),
                id3::Content::InvolvedPeopleList(list)
                    if frame.id() == "TMCL"
                        && list.items.len() == 2
                        && list.items[0].involvement == "guitar"
                        && list.items[0].involvee == "Alice"
                        && list.items[1].involvement == "vocals"
                        && list.items[1].involvee == "Bob"
            )
        }));
    }

    #[test]
    fn rejects_unwritable_or_underspecified_id3v24_edit_labels() {
        assert!(id3v24_edit_label_is_writable("TIT2"));
        assert!(id3v24_edit_label_is_writable(
            "TXXX:MusicIndex Contributors"
        ));
        assert!(id3v24_edit_label_is_writable("WXXX:Official audio"));
        assert!(id3v24_edit_label_is_writable("UFID:http://musicbrainz.org"));
        assert!(id3v24_edit_label_is_writable("APIC"));
        assert!(id3v24_edit_label_is_writable("COMM:MusicIndex Description"));
        assert!(id3v24_edit_label_is_writable("SYLT:MusicIndex Transcript"));
        assert!(id3v24_edit_label_is_writable("USLT:MusicIndex Transcript"));
        assert!(id3v24_edit_label_is_writable("TYER"));
        assert!(!id3v24_edit_label_is_writable("TXXX"));
        assert!(!id3v24_edit_label_is_writable("WXXX"));
        assert!(!id3v24_edit_label_is_writable("UFID"));
        assert!(!id3v24_edit_label_is_writable("COMM"));
        assert!(!id3v24_edit_label_is_writable("SYLT"));
        assert!(!id3v24_edit_label_is_writable("USLT"));
        assert!(!id3v24_edit_label_is_writable("TFOO"));

        let temp = tempfile::NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"not really an mp3").expect("write file");
        let edits = [Id3v24Edit {
            frame_label: "TFOO".into(),
            value: "not writable".into(),
        }];

        let error = write_id3v24_edits(temp.path(), &edits)
            .expect_err("unsupported ID3v2.4 edits should be rejected");
        assert!(
            error.to_string().contains("unsupported ID3v2.4"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn frame_descriptors_strip_nulls_and_whitespace() {
        assert_eq!(
            normalize_frame_descriptor(" \0MusicIndex Contributors\0 "),
            Some("MusicIndex Contributors".into())
        );
    }
}
