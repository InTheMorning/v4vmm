use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use id3::{no_tag_ok, Content, Tag, TagLike};

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

pub fn read_audio_tags(path: &Path) -> Result<AudioTags> {
    read_mp3_tags(path)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use id3::frame::{ExtendedText, Picture, PictureType};
    use id3::{Frame, Tag, TagLike};

    use super::{audio_tags_from_id3, read_audio_tags, AudioTags, EmbeddedArtwork, Id3Field};

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
}
