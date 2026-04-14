use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use id3::frame::{ExtendedLink, ExtendedText, UniqueFileIdentifier};
use id3::{no_tag_ok, Content, Frame, Tag, TagLike, Version};

const WRITABLE_TEXT_FRAMES: &[&str] = &[
    "TALB", "TBPM", "TCOM", "TCON", "TCOP", "TDEN", "TDLY", "TDOR", "TDRC", "TDRL", "TDTG", "TENC",
    "TEXT", "TFLT", "TIPL", "TIT1", "TIT2", "TIT3", "TKEY", "TLAN", "TLEN", "TMCL", "TMED", "TMOO",
    "TOAL", "TOFN", "TOLY", "TOPE", "TOWN", "TPE1", "TPE2", "TPE3", "TPE4", "TPOS", "TPRO", "TPUB",
    "TRCK", "TRSN", "TRSO", "TSOA", "TSOP", "TSOT", "TSRC", "TSSE", "TSST", "TXXX",
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
    tag.frames()
        .map(|frame| Id3Field {
            frame_id: frame.id().to_string(),
            value: frame.content().to_string(),
        })
        .collect()
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
    !matches!(frame_id.as_str(), "TXXX" | "WXXX" | "UFID") || descriptor.is_some()
}

fn id3v24_edit_frame_is_writable(frame_id: &str) -> bool {
    WRITABLE_TEXT_FRAMES.contains(&frame_id)
        || WRITABLE_URL_FRAMES.contains(&frame_id)
        || frame_id == "UFID"
}

fn required_frame_descriptor(frame_id: &str, descriptor: Option<String>) -> Result<String> {
    descriptor.ok_or_else(|| anyhow!("{frame_id} edits require a descriptor"))
}

fn split_frame_label(label: &str) -> (String, Option<String>) {
    let Some((frame_id, descriptor)) = label.split_once(':') else {
        return (label.trim().to_ascii_uppercase(), None);
    };
    let descriptor = descriptor.trim();
    (
        frame_id.trim().to_ascii_uppercase(),
        (!descriptor.is_empty()).then(|| descriptor.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use id3::frame::{ExtendedText, Picture, PictureType};
    use id3::{Frame, Tag, TagLike};

    use super::{
        audio_tags_from_id3, id3v24_edit_label_is_writable, read_audio_tags, write_id3v24_edits,
        AudioTags, EmbeddedArtwork, Id3Field, Id3v24Edit,
    };

    #[test]
    fn maps_id3_frames_to_audio_tags() {
        let mut tag = Tag::new();
        tag.set_title("Song Title");
        tag.set_artist("Track Artist");
        tag.set_album("Feed Title");
        tag.set_track(7);
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
                        value: "7".into(),
                    },
                    Id3Field {
                        frame_id: "TYER".into(),
                        value: "2026".into(),
                    },
                    Id3Field {
                        frame_id: "TXXX".into(),
                        value: "V4V_PUBLISHER: Wavlake".into(),
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

        let edits = [
            Id3v24Edit {
                frame_label: "TIT2".into(),
                value: "Song".into(),
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
                frame_label: "UFID:http://musicbrainz.org".into(),
                value: "recording-id".into(),
            },
        ];

        assert_eq!(
            write_id3v24_edits(temp.path(), &edits).expect("write ID3v2.4 edits"),
            4
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
        assert!(tag.frames().any(|frame| {
            frame.id() == "UFID"
                && frame.content().to_string() == "http://musicbrainz.org: recording-id"
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
        assert!(!id3v24_edit_label_is_writable("TXXX"));
        assert!(!id3v24_edit_label_is_writable("WXXX"));
        assert!(!id3v24_edit_label_is_writable("UFID"));
        assert!(!id3v24_edit_label_is_writable("APIC"));
        assert!(!id3v24_edit_label_is_writable("TFOO"));

        let temp = tempfile::NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"not really an mp3").expect("write file");
        let edits = [Id3v24Edit {
            frame_label: "APIC".into(),
            value: "not image data".into(),
        }];

        let error = write_id3v24_edits(temp.path(), &edits)
            .expect_err("non-simple ID3v2.4 edits should be rejected");
        assert!(
            error.to_string().contains("unsupported ID3v2.4"),
            "unexpected error: {error}"
        );
    }
}
