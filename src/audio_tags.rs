use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use id3::frame::{
    Comment, ExtendedLink, ExtendedText, InvolvedPeopleList, InvolvedPeopleListItem, Lyrics,
    Picture, PictureType, SynchronisedLyrics, SynchronisedLyricsType, TimestampFormat,
    UniqueFileIdentifier,
};
use id3::{no_tag_ok, Content, Frame, Tag, TagLike, Version};

use crate::media::image_type;
use crate::remote_media;

const WRITABLE_TEXT_FRAMES: &[&str] = &[
    "TALB", "TBPM", "TCOM", "TCON", "TCOP", "TDEN", "TDLY", "TDOR", "TDRC", "TDRL", "TDTG", "TENC",
    "TEXT", "TFLT", "TIT1", "TIT2", "TIT3", "TKEY", "TLAN", "TLEN", "TMED", "TMOO", "TOAL", "TOFN",
    "TOLY", "TOPE", "TOWN", "TPE1", "TPE2", "TPE3", "TPE4", "TPOS", "TPRO", "TPUB", "TRCK", "TRSN",
    "TRSO", "TSOA", "TSOP", "TSOT", "TSRC", "TSSE", "TSST", "TXXX", "TYER",
];
const WRITABLE_URL_FRAMES: &[&str] = &[
    "WCOM", "WCOP", "WOAF", "WOAR", "WOAS", "WORS", "WPAY", "WPUB", "WXXX",
];

/// ADR 0080 Decision 2: the two frame labels that hold a website. A write
/// treats them as one family: a value under one label can replace an
/// earlier value under either label, so a fact that moved frame, or a
/// repeated write, leaves no stale copy (Decision 6).
const WEBSITE_FRAME_LABELS: [&str; 2] = ["WOAR", "WOAF"];

/// ADR 0080 Decision 5: the two frame labels that hold a description. A
/// track holds at most one of them, so a write that sets either one clears
/// both first.
const DESCRIPTION_FRAME_LABELS: [&str; 2] = [
    "COMM:MusicIndex Description",
    "COMM:MusicIndex Album Description",
];

/// The plain, lower-cased URL that `edits` is about to write into a `WOAR`
/// or `WOAF` frame. Empty when this write touches neither frame.
fn website_urls_in_edits(edits: &[Id3v24Edit]) -> BTreeSet<String> {
    edits
        .iter()
        .filter(|edit| WEBSITE_FRAME_LABELS.contains(&edit.frame_label.as_str()))
        .map(|edit| embedded_frame_url(&edit.value).to_ascii_lowercase())
        .collect()
}

/// The URL embedded in a `WOAR` or `WOAF` value, with a label an earlier
/// write put before it removed (ADR 0080 Decision 6). A value with no label
/// is already a plain URL.
fn embedded_frame_url(value: &str) -> &str {
    match value.find("https://").or_else(|| value.find("http://")) {
        Some(start) => value[start..].trim(),
        None => value.trim(),
    }
}

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
    use crate::audio_format::AudioFormat;
    match AudioFormat::detect_from_file(path) {
        Ok(AudioFormat::Mp3) => read_mp3_tags(path),
        Ok(_) => read_lofty_tags(path),
        // Unknown format — fall back to id3 so we don't regress on files whose
        // magic bytes weren't recognised (e.g. unusual MP3 headers).
        Err(_) => read_mp3_tags(path),
    }
}

fn read_lofty_tags(path: &Path) -> Result<AudioTags> {
    use lofty::file::TaggedFileExt;
    use lofty::prelude::{Accessor, ItemKey};
    use lofty::probe::Probe;
    use lofty::tag::{ItemValue, Tag as LoftyTag};

    let tagged = Probe::open(path)
        .with_context(|| format!("probe {}", path.display()))?
        .read()
        .with_context(|| format!("read tags from {}", path.display()))?;

    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(AudioTags::default());
    };

    fn cow_to_string(value: Option<std::borrow::Cow<'_, str>>) -> Option<String> {
        value.map(|c| c.into_owned())
    }

    let title = cow_to_string(Accessor::title(tag));
    let artist = cow_to_string(Accessor::artist(tag));
    let album = cow_to_string(Accessor::album(tag));
    let track_number = Accessor::track(tag).map(|n| n.to_string());
    let total_tracks = Accessor::track_total(tag).map(|n| n.to_string());
    let date = LoftyTag::get_string(tag, &ItemKey::RecordingDate)
        .map(|s| s.to_string())
        .or_else(|| Accessor::year(tag).map(|y| y.to_string()));

    let mut custom = BTreeMap::new();
    let mut fields: Vec<Id3Field> = Vec::new();
    for item in tag.items() {
        let key_label = lofty_item_label(item.key());
        let value = match item.value() {
            ItemValue::Text(text) | ItemValue::Locator(text) => text.clone(),
            ItemValue::Binary(_) => continue,
        };
        if let ItemKey::Unknown(name) = item.key() {
            custom.insert(name.clone(), value.clone());
        }
        fields.push(Id3Field {
            frame_id: key_label,
            value,
        });
    }

    let artwork = tag.pictures().first().map(|pic| EmbeddedArtwork {
        mime_type: pic
            .mime_type()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "application/octet-stream".into()),
        picture_type: format!("{:?}", pic.pic_type()),
        description: pic.description().unwrap_or("").to_string(),
        data: pic.data().to_vec(),
    });

    add_lofty_compare_aliases(&mut fields, artwork.as_ref());

    Ok(AudioTags {
        title,
        artist,
        album,
        track_number,
        total_tracks,
        date,
        custom,
        artwork,
        fields,
    })
}

fn add_lofty_compare_aliases(fields: &mut Vec<Id3Field>, artwork: Option<&EmbeddedArtwork>) {
    if let Some(comment) = first_field_value(fields, "COMM") {
        push_alias_field(fields, "COMM:MusicIndex Description", comment);
    }
    add_lofty_album_description_alias(fields);

    if let Some(transcript) =
        first_field_value(fields, "USLT").or_else(|| first_field_value(fields, "SYLT"))
    {
        push_alias_field(fields, "USLT:MusicIndex Transcript", transcript.clone());
        push_alias_field(fields, "SYLT:MusicIndex Transcript", transcript);
    }

    if let Some(artwork) = artwork {
        let summary = if artwork.description.trim().is_empty() {
            format!(
                "{} ({}, {} bytes)",
                artwork.picture_type,
                artwork.mime_type,
                artwork.data.len()
            )
        } else {
            format!(
                "{}: {} ({}, {} bytes)",
                artwork.description,
                artwork.picture_type,
                artwork.mime_type,
                artwork.data.len()
            )
        };
        push_alias_field(fields, "APIC", summary);
    }
}

/// ADR 0080 Decisions 3 and 5: on Vorbis Comments and MP4 freeform atoms,
/// the album description round-trips as a `TXXX`-style field, the same way
/// any other named `TXXX` descriptor does (`lofty_field_for_label` gives it
/// that key when the writer writes it). This aliases the field back to
/// `COMM:MusicIndex Album Description`, so the compare grid and the tag
/// scan find it under the frame the writer gives it.
fn add_lofty_album_description_alias(fields: &mut Vec<Id3Field>) {
    let Some(value) = fields.iter().find_map(|field| {
        let descriptor = field.frame_id.strip_prefix("TXXX:")?;
        descriptor
            .eq_ignore_ascii_case("MusicIndex Album Description")
            .then(|| field.value.clone())
    }) else {
        return;
    };
    push_alias_field(fields, "COMM:MusicIndex Album Description", value);
}

fn push_alias_field(fields: &mut Vec<Id3Field>, frame_id: &str, value: String) {
    if fields.iter().any(|field| field.frame_id == frame_id) {
        return;
    }
    fields.push(Id3Field {
        frame_id: frame_id.to_string(),
        value,
    });
}

fn first_field_value(fields: &[Id3Field], frame_base: &str) -> Option<String> {
    fields.iter().find_map(|field| {
        field
            .frame_id
            .split(':')
            .next()
            .is_some_and(|base| base == frame_base)
            .then(|| field.value.clone())
    })
}

/// Map a lofty `ItemKey` (Vorbis Comment / MP4 atom / etc.) onto the
/// equivalent ID3v2.4 frame label so the metadata comparator — which speaks
/// ID3 frame IDs natively — can match values regardless of source container.
///
/// Without this, FLAC/OGG/MP4 reads emit Debug strings like `"TrackTitle"`
/// that no comparator row recognises, making every populated field look
/// like an unapplied pending edit.
fn lofty_item_label(key: &lofty::prelude::ItemKey) -> String {
    use crate::tag_field::TagFieldId;
    use lofty::prelude::ItemKey;
    use lofty::tag::TagType;
    match key {
        ItemKey::Unknown(name) => TagFieldId::from_storage_key_name(name)
            .map(|field| match field {
                TagFieldId::Custom(desc) => format!("TXXX:{desc}"),
                TagFieldId::Url(kind) => kind.to_id3().to_string(),
                TagFieldId::Title => "TIT2".into(),
                TagFieldId::Artist => "TPE1".into(),
                TagFieldId::AlbumArtist => "TPE2".into(),
                TagFieldId::Album => "TALB".into(),
                TagFieldId::TrackNumber | TagFieldId::TotalTracks => "TRCK".into(),
                TagFieldId::DiscNumber => "TPOS".into(),
                TagFieldId::Date => "TDRC".into(),
                TagFieldId::Composer => "TCOM".into(),
                TagFieldId::Genre => "TCON".into(),
                TagFieldId::Publisher => "TPUB".into(),
                TagFieldId::Isrc => "TSRC".into(),
                TagFieldId::Comment => "COMM".into(),
                TagFieldId::Lyrics => "USLT".into(),
                TagFieldId::Id3Text(label)
                | TagFieldId::Id3Url(label)
                | TagFieldId::Id3Raw(label) => label,
            })
            .unwrap_or_else(|| format!("TXXX:{name}")),
        other => other
            .map_key(TagType::Id3v2, false)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{other:?}")),
    }
}

/// The result of one tag write (ADR 0080 task 004).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TagWriteResult {
    /// The number of edits that the write applied.
    pub applied: usize,
    /// The ID of each old frame that the write removed, because an ID3v2.4
    /// tag cannot hold its ID.
    pub removed_frames: Vec<String>,
}

pub fn write_id3v24_edits(path: &Path, edits: &[Id3v24Edit]) -> Result<TagWriteResult> {
    if edits.is_empty() {
        return Ok(TagWriteResult::default());
    }

    use crate::audio_format::AudioFormat;
    match AudioFormat::detect_from_file(path) {
        Ok(AudioFormat::Mp3) | Err(_) => write_mp3_edits(path, edits),
        Ok(AudioFormat::Flac) | Ok(AudioFormat::OggVorbis) | Ok(AudioFormat::OggOpus) => {
            write_lofty_edits(path, edits, lofty::tag::TagType::VorbisComments).map(lofty_result)
        }
        Ok(AudioFormat::Mp4) => {
            write_lofty_edits(path, edits, lofty::tag::TagType::Mp4Ilst).map(lofty_result)
        }
        Ok(AudioFormat::Wav) => Err(anyhow!(
            "cannot tag raw WAV ({}); re-subscribe with `flac` installed to upgrade",
            path.display()
        )),
    }
}

fn lofty_result(applied: usize) -> TagWriteResult {
    TagWriteResult {
        applied,
        removed_frames: Vec::new(),
    }
}

fn write_mp3_edits(path: &Path, edits: &[Id3v24Edit]) -> Result<TagWriteResult> {
    let mut tag = no_tag_ok(Tag::read_from_path(path))
        .with_context(|| format!("read embedded MP3 tags from {}", path.display()))?
        .unwrap_or_default();
    let removed_frames = convert_old_v22_frames(&mut tag);
    remove_stale_id3_website_frames(&mut tag, edits);
    remove_stale_id3_description_frames(&mut tag, edits);
    let mut applied = 0;

    for edit in edits {
        let frame = id3v24_edit_frame(edit)?;
        tag.add_frame(frame);
        applied += 1;
    }

    write_id3_tag_safely(path, &tag)?;
    Ok(TagWriteResult {
        applied,
        removed_frames,
    })
}

/// The ID3v2.2 frames from iTunes that the conversion table of the id3
/// crate does not convert, with their ID3v2.4 IDs (ADR 0080 task 004).
const OLD_ITUNES_FRAME_IDS: [(&str, &str); 6] = [
    ("TSP", "TSOP"),
    ("TSA", "TSOA"),
    ("TST", "TSOT"),
    ("TS2", "TSO2"),
    ("TSC", "TSOC"),
    ("TCP", "TCMP"),
];

/// ADR 0080 Decision 6: a write keeps each value from another tool. This
/// function gives each old iTunes frame its ID3v2.4 ID and keeps its value.
/// It removes each other frame whose ID is not 4 bytes long, because an
/// ID3v2.4 tag cannot hold that frame. It returns the ID of each removed
/// frame.
fn convert_old_v22_frames(tag: &mut Tag) -> Vec<String> {
    let mut removed = Vec::new();
    let mut converted = Vec::new();
    tag.frames_vec_mut().retain(|frame| {
        if frame.id().len() == 4 {
            return true;
        }
        match OLD_ITUNES_FRAME_IDS
            .iter()
            .find(|(old_id, _)| *old_id == frame.id())
        {
            Some((_, new_id)) => {
                converted.push(Frame::with_content(*new_id, frame.content().clone()));
            }
            None => removed.push(frame.id().to_owned()),
        }
        false
    });
    for frame in converted {
        tag.add_frame(frame);
    }
    removed
}

/// ADR 0080 task 004: a failed write never changes the file. The writer
/// encodes the tag into memory first. It then writes the tag into a staged
/// copy in the same directory and renames the copy over the file. The copy
/// keeps the file mode. The id3 crate writes the copy, so the bytes after
/// the tag stay equal to the bytes of the original file.
fn write_id3_tag_safely(path: &Path, tag: &Tag) -> Result<()> {
    tag.write_to(std::io::sink(), Version::Id3v24)
        .with_context(|| format!("encode ID3v2.4 tags for {}", path.display()))?;
    let target = fs::canonicalize(path)
        .with_context(|| format!("resolve the tag write target {}", path.display()))?;
    let staged = staged_tag_path(&target)?;
    let written = fs::copy(&target, &staged)
        .with_context(|| format!("copy {} to {}", target.display(), staged.display()))
        .and_then(|_| {
            tag.write_to_path(&staged, Version::Id3v24)
                .with_context(|| format!("write ID3v2.4 tags to {}", staged.display()))
        })
        .and_then(|()| {
            fs::rename(&staged, &target)
                .with_context(|| format!("replace {} with {}", target.display(), staged.display()))
        });
    if written.is_err() {
        let _ = fs::remove_file(&staged);
    }
    written.with_context(|| format!("write ID3v2.4 tags to {}", path.display()))
}

/// A path for the staged copy of `target`, in the directory of `target`.
fn staged_tag_path(target: &Path) -> Result<std::path::PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let directory = target
        .parent()
        .with_context(|| format!("find the directory of {}", target.display()))?;
    let name = target
        .file_name()
        .with_context(|| format!("find the file name of {}", target.display()))?;
    let mut staged_name = std::ffi::OsString::from(".");
    staged_name.push(name);
    staged_name.push(format!(
        ".v4vmm-tag-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(directory.join(staged_name))
}

/// ADR 0080 Decision 6: a write removes a `WOAR` or `WOAF` value under
/// either label when the same URL is about to be written again. This
/// recognizes the earlier labeled channel value and the item page
/// mistakenly held in `WOAR`, and removes only those. A value from another
/// tool, or a MusicBrainz value this write does not repeat, is not touched.
fn remove_stale_id3_website_frames(tag: &mut Tag, edits: &[Id3v24Edit]) {
    let new_urls = website_urls_in_edits(edits);
    if new_urls.is_empty() {
        return;
    }
    tag.frames_vec_mut().retain(|frame| {
        if !WEBSITE_FRAME_LABELS.contains(&frame.id()) {
            return true;
        }
        match frame.content().link() {
            Some(link) => !new_urls.contains(&embedded_frame_url(link).to_ascii_lowercase()),
            None => true,
        }
    });
}

/// ADR 0080 Decisions 5 and 6: `COMM:MusicIndex Description` and
/// `COMM:MusicIndex Album Description` hold at most one value between them.
/// When this write sets either one, it clears both first, so a fact that
/// moved descriptor leaves no stale copy under its earlier one.
fn remove_stale_id3_description_frames(tag: &mut Tag, edits: &[Id3v24Edit]) {
    let touches_description = edits
        .iter()
        .any(|edit| DESCRIPTION_FRAME_LABELS.contains(&edit.frame_label.as_str()));
    if !touches_description {
        return;
    }
    tag.frames_vec_mut().retain(|frame| {
        let Content::Comment(comment) = frame.content() else {
            return true;
        };
        let label = format!("COMM:{}", comment.description);
        !DESCRIPTION_FRAME_LABELS.contains(&label.as_str())
    });
}

fn write_lofty_edits(
    path: &Path,
    edits: &[Id3v24Edit],
    tag_type: lofty::tag::TagType,
) -> Result<usize> {
    use lofty::file::{AudioFile, TaggedFileExt};
    use lofty::picture::{MimeType, Picture, PictureType};
    use lofty::prelude::{Accessor, ItemKey};
    use lofty::probe::Probe;
    use lofty::tag::{ItemValue, Tag as LoftyTag, TagItem};

    use crate::tag_field::TagFieldId;

    let mut tagged = Probe::open(path)
        .with_context(|| format!("probe {}", path.display()))?
        .read()
        .with_context(|| format!("read tags from {}", path.display()))?;

    let mut tag = tagged
        .remove(tag_type)
        .unwrap_or_else(|| LoftyTag::new(tag_type));
    // Each cleanup pass runs once, before any edit is added. A pass inside
    // the loop would remove the value of an earlier edit that shares its
    // key with a later one (orchestrator review, defect 2).
    remove_stale_lofty_website_items(&mut tag, tag_type, &website_urls_in_edits(edits));
    remove_stale_lofty_description_items(&mut tag, tag_type, edits);
    remove_stale_lofty_keyed_items(&mut tag, tag_type, edits);

    let mut applied = 0usize;
    for edit in edits {
        // Artwork lands as a Picture rather than a text item.
        if edit.frame_label.starts_with("APIC") {
            match read_picture_reference(&edit.value) {
                Ok((mime, data)) => {
                    tag.push_picture(Picture::new_unchecked(
                        PictureType::CoverFront,
                        Some(MimeType::from_str(&mime)),
                        None,
                        data,
                    ));
                    applied += 1;
                }
                Err(err) => {
                    eprintln!("skip artwork edit for {}: {err:#}", path.display());
                }
            }
            continue;
        }

        let field = lofty_field_for_label(&edit.frame_label);
        let inserted = match tag_type {
            lofty::tag::TagType::VorbisComments => {
                let Some(key) = field.vorbis_key() else {
                    continue;
                };
                // `Tag::push` rejects `ItemKey::Unknown` because re_map
                // fails for keys without a built-in mapping, so this uses
                // the unchecked push instead. The cleanup above already
                // removed one item for each key this write owns, so a
                // plain push here does not duplicate on a repeated write,
                // and two edits that share a key (`TDRC` and `TYER`, both
                // `DATE`) both land.
                tag.push_unchecked(TagItem::new(
                    ItemKey::Unknown(key),
                    ItemValue::Text(edit.value.clone()),
                ));
                true
            }
            lofty::tag::TagType::Mp4Ilst => {
                let handled = match &field {
                    TagFieldId::Title => {
                        Accessor::set_title(&mut tag, edit.value.clone());
                        true
                    }
                    TagFieldId::Artist => {
                        Accessor::set_artist(&mut tag, edit.value.clone());
                        true
                    }
                    TagFieldId::Album => {
                        Accessor::set_album(&mut tag, edit.value.clone());
                        true
                    }
                    TagFieldId::TrackNumber => {
                        if let Ok(n) = edit.value.parse::<u32>() {
                            Accessor::set_track(&mut tag, n);
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                if !handled {
                    let ns_key = lofty_mp4_freeform_key(&edit.frame_label, &field);
                    tag.push_unchecked(TagItem::new(
                        ItemKey::Unknown(ns_key),
                        ItemValue::Text(edit.value.clone()),
                    ));
                }
                true
            }
            _ => false,
        };
        if inserted {
            applied += 1;
        }
    }

    tagged.insert_tag(tag);
    tagged
        .save_to_path(path, lofty::config::WriteOptions::default())
        .with_context(|| format!("write tags to {}", path.display()))?;
    Ok(applied)
}

/// ADR 0080 Decision 5: the field that `frame_label` maps to, for the lofty
/// writer. `COMM:MusicIndex Album Description` gets its own key, the same
/// way a `TXXX` descriptor does, so it does not collide with the shared
/// Comment key (Vorbis `COMMENT`, MP4 `©cmt`) that
/// `COMM:MusicIndex Description` keeps.
fn lofty_field_for_label(frame_label: &str) -> crate::tag_field::TagFieldId {
    use crate::tag_field::TagFieldId;
    if frame_label == "COMM:MusicIndex Album Description" {
        TagFieldId::Custom("MusicIndex Album Description".into())
    } else {
        TagFieldId::from_id3_label(frame_label)
    }
}

/// The MP4 freeform atom name that `field` stores under, once `field` is
/// known not to be one of the atoms an MP4 accessor sets directly.
fn lofty_mp4_freeform_key(frame_label: &str, field: &crate::tag_field::TagFieldId) -> String {
    use crate::tag_field::TagFieldId;
    match field {
        TagFieldId::Custom(desc) => format!("----:com.apple.iTunes:{desc}"),
        TagFieldId::Url(kind) => format!("----:com.apple.iTunes:{}", kind.to_id3()),
        _ => format!("----:com.apple.iTunes:{frame_label}"),
    }
}

/// The key that `frame_label` stores under, in `tag_type`'s tag, through
/// the generic push path. `None` for a Vorbis comment with no mapping for
/// this field, and for a tag type this writer does not handle.
fn lofty_generic_key(frame_label: &str, tag_type: lofty::tag::TagType) -> Option<String> {
    let field = lofty_field_for_label(frame_label);
    match tag_type {
        lofty::tag::TagType::VorbisComments => field.vorbis_key(),
        lofty::tag::TagType::Mp4Ilst => Some(lofty_mp4_freeform_key(frame_label, &field)),
        _ => None,
    }
}

/// ADR 0080 Decision 6: the Vorbis Comment key that `frame_label` stores
/// under in a FLAC, Ogg Vorbis or Ogg Opus file. The writer uses the same
/// key. Two labels can share one key, for example `TDRC` and `TYER`, both
/// `DATE`. The function gives `None` for other formats and for a label
/// without a Vorbis key.
pub(crate) fn vorbis_storage_key(
    frame_label: &str,
    format: crate::audio_format::AudioFormat,
) -> Option<String> {
    use crate::audio_format::AudioFormat;
    match format {
        AudioFormat::Flac | AudioFormat::OggVorbis | AudioFormat::OggOpus => {
            lofty_generic_key(frame_label, lofty::tag::TagType::VorbisComments)
                .map(|key| key.to_ascii_uppercase())
        }
        AudioFormat::Mp3 | AudioFormat::Mp4 | AudioFormat::Wav => None,
    }
}

/// `true` for the two logical fields that map to `WOAR` and `WOAF`. A
/// website edit is not part of the once-per-key removal that
/// `remove_stale_lofty_keyed_items` does: `WOAR` can hold more than one
/// value (the channel website and a MusicBrainz value), and
/// `remove_stale_lofty_website_items` already covers the value this write
/// repeats.
fn is_website_field(field: &crate::tag_field::TagFieldId) -> bool {
    use crate::tag_field::{TagFieldId, UrlKind};
    matches!(
        field,
        TagFieldId::Url(UrlKind::OfficialArtist) | TagFieldId::Url(UrlKind::OfficialAudio)
    )
}

/// Removes each existing item whose stored key, under `tag_type`, matches
/// `key`. Lofty can read a standard key (`TITLE`, for example) back as its
/// own typed `ItemKey` rather than as `ItemKey::Unknown`, so comparing the
/// mapped key string, instead of the `ItemKey` value, keeps one item for
/// that key across a repeated write (ADR 0080 Decision 6).
fn remove_lofty_item_by_key(tag: &mut lofty::tag::Tag, tag_type: lofty::tag::TagType, key: &str) {
    tag.retain(|item| {
        item.key()
            .map_key(tag_type, true)
            .is_none_or(|existing| !existing.eq_ignore_ascii_case(key))
    });
}

/// ADR 0080 Decision 6: removes each key that `edits` gives a non-website,
/// non-artwork field, once, before any edit of this write is added.
///
/// A pass inside the write loop would remove one edit's value when a later
/// edit that shares its key was added (orchestrator review, defect 2): two
/// edits can share a key, for example `TDRC` and `TYER`, both `DATE` on
/// Vorbis Comments. Clearing each owned key once, first, keeps both edits'
/// values, and still keeps a repeated write from duplicating either one.
fn remove_stale_lofty_keyed_items(
    tag: &mut lofty::tag::Tag,
    tag_type: lofty::tag::TagType,
    edits: &[Id3v24Edit],
) {
    use crate::tag_field::TagFieldId;

    let mut keys = BTreeSet::new();
    for edit in edits {
        if edit.frame_label.starts_with("APIC") {
            continue;
        }
        let field = lofty_field_for_label(&edit.frame_label);
        if is_website_field(&field) {
            continue;
        }
        // An MP4 accessor sets this field directly; it is not a keyed push.
        if tag_type == lofty::tag::TagType::Mp4Ilst
            && matches!(
                field,
                TagFieldId::Title
                    | TagFieldId::Artist
                    | TagFieldId::Album
                    | TagFieldId::TrackNumber
            )
        {
            continue;
        }
        if let Some(key) = lofty_generic_key(&edit.frame_label, tag_type) {
            keys.insert(key);
        }
    }
    for key in keys {
        remove_lofty_item_by_key(tag, tag_type, &key);
    }
}

/// ADR 0080 Decisions 3 and 5, for Vorbis Comments and MP4 freeform atoms:
/// the item's own description and the channel's hold at most one value
/// between them. `COMM:MusicIndex Album Description` keeps its own key
/// apart from the shared Comment key that `COMM:MusicIndex Description`
/// keeps (`lofty_field_for_label`), so the two no longer collide, and a
/// file can now carry the channel's description on these formats. When
/// this write sets either label, it clears the other's stored key first,
/// so a fact that moved owner leaves no stale copy under its earlier key.
fn remove_stale_lofty_description_items(
    tag: &mut lofty::tag::Tag,
    tag_type: lofty::tag::TagType,
    edits: &[Id3v24Edit],
) {
    let touches_description = edits
        .iter()
        .any(|edit| DESCRIPTION_FRAME_LABELS.contains(&edit.frame_label.as_str()));
    if !touches_description {
        return;
    }
    for label in DESCRIPTION_FRAME_LABELS {
        if let Some(key) = lofty_generic_key(label, tag_type) {
            remove_lofty_item_by_key(tag, tag_type, &key);
        }
    }
}

/// ADR 0080 Decision 6: removes a Vorbis comment or MP4 freeform item under
/// the stored key of `WOAR` or `WOAF` whose URL is about to be written
/// again, so a value that moved frame, or a repeated write, leaves no stale
/// item behind. An item under another key, or a website item with a
/// different URL, is not touched.
fn remove_stale_lofty_website_items(
    tag: &mut lofty::tag::Tag,
    tag_type: lofty::tag::TagType,
    new_urls: &BTreeSet<String>,
) {
    use lofty::prelude::ItemKey;
    use lofty::tag::ItemValue;

    if new_urls.is_empty() {
        return;
    }
    let keys = lofty_website_item_keys(tag_type);
    tag.retain(|item| {
        let ItemKey::Unknown(name) = item.key() else {
            return true;
        };
        if !keys.iter().any(|key| key.eq_ignore_ascii_case(name)) {
            return true;
        }
        match item.value() {
            ItemValue::Text(text) => {
                !new_urls.contains(&embedded_frame_url(text).to_ascii_lowercase())
            }
            _ => true,
        }
    });
}

/// The stored item key that holds `WOAR` and `WOAF` in `tag_type`.
fn lofty_website_item_keys(tag_type: lofty::tag::TagType) -> [String; 2] {
    use crate::tag_field::{TagFieldId, UrlKind};

    match tag_type {
        lofty::tag::TagType::VorbisComments => [
            TagFieldId::Url(UrlKind::OfficialArtist)
                .vorbis_key()
                .unwrap_or_default(),
            TagFieldId::Url(UrlKind::OfficialAudio)
                .vorbis_key()
                .unwrap_or_default(),
        ],
        _ => [
            format!("----:com.apple.iTunes:{}", UrlKind::OfficialArtist.to_id3()),
            format!("----:com.apple.iTunes:{}", UrlKind::OfficialAudio.to_id3()),
        ],
    }
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
                .unwrap_or_else(|_| format!("{:x?}", ufid.identifier)),
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

pub(crate) fn sanitize_title_text(value: &str) -> String {
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
        let response = remote_media::fetch(reference, "transcript")?;
        // A redirect landing page answers with 200 and markup. Without this the
        // page body is embedded into the file's tags as transcript text.
        if let Some(content_type) = remote_media::declared_content_type(&response) {
            if content_type.starts_with("text/html") || content_type.contains("xhtml") {
                return Err(anyhow!(
                    "transcript {reference} returned markup ({content_type}), not transcript text"
                ));
            }
        }
        return response
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
        let response = remote_media::fetch(reference, "APIC image")?;
        let declared_mime_type = remote_media::declared_content_type(&response);
        let data = response
            .bytes()
            .with_context(|| format!("read APIC image {reference}"))?
            .to_vec();
        if data.is_empty() {
            return Err(anyhow!("APIC image is empty"));
        }
        // Bytes only, no declared-type fallback. APIC writes an artifact, so a
        // 200 response carrying markup under `Content-Type: image/jpeg` must not
        // become the picture frame. Display paths may fall back to the declared
        // type because their worst case is a broken thumbnail (ADR 0056).
        let mime_type = image_type::from_bytes(&data).ok_or_else(|| {
            anyhow!(
                "APIC image response missing image type (declared {})",
                declared_mime_type.as_deref().unwrap_or("nothing")
            )
        })?;
        return Ok((mime_type, data));
    }

    let path = Path::new(reference);
    let data = fs::read(path).with_context(|| format!("read APIC image {}", path.display()))?;
    if data.is_empty() {
        return Err(anyhow!("APIC image is empty"));
    }
    let mime_type = image_type::from_path(path)
        .or_else(|| image_type::from_bytes(&data))
        .ok_or_else(|| anyhow!("unsupported APIC image type for {}", path.display()))?;
    Ok((mime_type, data))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use id3::frame::{ExtendedText, Picture, PictureType};
    use id3::{Frame, Tag, TagLike};
    use lofty::prelude::ItemKey;

    use super::{
        add_lofty_compare_aliases, audio_tags_from_id3, id3v24_edit_label_is_writable,
        lofty_item_label, normalize_frame_descriptor, read_audio_tags, read_picture_reference,
        read_text_reference, write_id3v24_edits, AudioTags, EmbeddedArtwork, Id3Field, Id3v24Edit,
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
    fn lofty_unknown_artist_webpage_maps_back_to_woar() {
        assert_eq!(
            lofty_item_label(&ItemKey::Unknown("ARTISTWEBPAGE".into())),
            "WOAR"
        );
        assert_eq!(
            lofty_item_label(&ItemKey::Unknown("----:com.apple.iTunes:WOAR".into())),
            "WOAR"
        );
    }

    #[test]
    fn lofty_aliases_fill_descriptor_and_artwork_presence_gaps() {
        let mut fields = vec![
            Id3Field {
                frame_id: "COMM".into(),
                value: "MusicIndex description".into(),
            },
            Id3Field {
                frame_id: "USLT".into(),
                value: "Embedded transcript".into(),
            },
        ];
        let artwork = EmbeddedArtwork {
            mime_type: "image/jpeg".into(),
            picture_type: "CoverFront".into(),
            description: "front cover".into(),
            data: vec![1, 2, 3],
        };

        add_lofty_compare_aliases(&mut fields, Some(&artwork));

        assert!(fields
            .iter()
            .any(|field| field.frame_id == "COMM:MusicIndex Description"));
        assert!(fields
            .iter()
            .any(|field| field.frame_id == "USLT:MusicIndex Transcript"));
        assert!(fields
            .iter()
            .any(|field| field.frame_id == "SYLT:MusicIndex Transcript"));
        assert!(fields.iter().any(|field| field.frame_id == "APIC"));
    }

    /// A redirect landing page answers with 200 and markup. Without the
    /// content-type rule it is embedded into the file's tags as transcript
    /// text (ADR 0056).
    #[test]
    fn transcript_download_rejects_markup_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 18\r\nConnection: close\r\n\r\n<html>moved</html>",
            )
            .expect("write response");
        });

        let error = read_text_reference(&format!("http://{addr}/transcript.txt"))
            .expect_err("markup must not become transcript text");

        assert!(
            error.to_string().contains("returned markup"),
            "error should explain the markup rejection: {error}"
        );
    }

    #[test]
    fn apic_image_download_follows_redirect_with_spaces() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut redirect_stream, _) = listener.accept().expect("accept redirect request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut redirect_stream, &mut buf);
            let location = format!("http://{addr}/Assets/front cover.jpg");
            let response = format!(
                "HTTP/1.1 301 Moved Permanently\r\nLocation: {location}\r\nContent-Type: text/html\r\nContent-Length: 8\r\nConnection: close\r\n\r\nredirect"
            );
            std::io::Write::write_all(&mut redirect_stream, response.as_bytes())
                .expect("write redirect response");

            let (mut image_stream, _) = listener.accept().expect("accept image request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut image_stream, &mut buf);
            std::io::Write::write_all(
                &mut image_stream,
                b"HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: 13\r\nConnection: close\r\n\r\n\xff\xd8\xffjpeg bytes",
            )
            .expect("write image response");
        });

        let (mime_type, data) =
            read_picture_reference(&format!("http://{addr}/cover.jpg")).expect("read APIC image");

        assert_eq!(mime_type, "image/jpeg");
        assert_eq!(data, b"\xff\xd8\xffjpeg bytes");
    }

    /// APIC is stricter than the display paths: a declared image type on a 200
    /// response is not enough to embed artwork. Before this rule a lying server
    /// could put markup into the picture frame (ADR 0056).
    #[test]
    fn apic_image_download_rejects_markup_declared_as_an_image() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: 18\r\nConnection: close\r\n\r\n<html>moved</html>",
            )
            .expect("write response");
        });

        let error = read_picture_reference(&format!("http://{addr}/cover.jpg"))
            .expect_err("a declared image type must not be enough to embed artwork");

        assert!(
            error.to_string().contains("missing image type"),
            "error should explain the byte-recognition failure: {error}"
        );
    }

    #[test]
    fn apic_image_download_rejects_non_image_redirect_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buf = [0_u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 17\r\nConnection: close\r\n\r\n<html>moved</html>",
            )
            .expect("write response");
        });

        let error = read_picture_reference(&format!("http://{addr}/cover.jpg"))
            .expect_err("non-image response should not become APIC artwork");

        assert!(
            error.to_string().contains("missing image type"),
            "error should explain image type validation: {error}"
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
            write_id3v24_edits(temp.path(), &edits)
                .expect("write ID3v2.4 edits")
                .applied,
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

    fn woar_frame_values(tag: &Tag) -> Vec<String> {
        let mut values = tag
            .frames()
            .filter(|frame| frame.id() == "WOAR")
            .map(|frame| frame.content().link().unwrap_or_default().to_string())
            .collect::<Vec<_>>();
        values.sort();
        values
    }

    fn woaf_frame_values(tag: &Tag) -> Vec<String> {
        tag.frames()
            .filter(|frame| frame.id() == "WOAF")
            .map(|frame| frame.content().link().unwrap_or_default().to_string())
            .collect::<Vec<_>>()
    }

    /// R80-06: a write of the same edits twice to an MP3 file gives equal
    /// frames. The second write adds no duplicate.
    #[test]
    fn adr_0080_repeated_mp3_write_gives_equal_frames() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"not really an mp3").expect("write file");
        let edits = [
            Id3v24Edit {
                frame_label: "TIT2".into(),
                value: "Song".into(),
            },
            Id3v24Edit {
                frame_label: "TALB".into(),
                value: "Feed Title".into(),
            },
            Id3v24Edit {
                frame_label: "TXXX:RSS Nostr Handle".into(),
                value: "npub1example".into(),
            },
            Id3v24Edit {
                frame_label: "WOAR".into(),
                value: "https://example.test/feed".into(),
            },
            Id3v24Edit {
                frame_label: "WOAF".into(),
                value: "https://example.test/track".into(),
            },
            Id3v24Edit {
                frame_label: "COMM:MusicIndex Description".into(),
                value: "Track description".into(),
            },
        ];

        write_id3v24_edits(temp.path(), &edits).expect("first write");
        let first = read_audio_tags(temp.path())
            .expect("read after first write")
            .fields;
        write_id3v24_edits(temp.path(), &edits).expect("second write");
        let second = read_audio_tags(temp.path())
            .expect("read after second write")
            .fields;

        assert_eq!(first, second, "a repeated write must not duplicate a frame");
    }

    /// R80-06: the same idempotent-write proof for FLAC, on a copy of a real
    /// FLAC fixture in a temporary file.
    #[test]
    fn adr_0080_repeated_flac_write_gives_equal_fields() {
        let flac_bytes = include_bytes!("../docs/runbooks/fixtures/conversion.flac");
        let temp = tempfile::Builder::new()
            .suffix(".flac")
            .tempfile()
            .expect("temp flac file");
        fs::write(temp.path(), flac_bytes).expect("write flac fixture copy");
        let edits = [
            Id3v24Edit {
                frame_label: "TIT2".into(),
                value: "Song".into(),
            },
            Id3v24Edit {
                frame_label: "TALB".into(),
                value: "Feed Title".into(),
            },
            Id3v24Edit {
                frame_label: "TXXX:RSS Nostr Handle".into(),
                value: "npub1example".into(),
            },
            Id3v24Edit {
                frame_label: "WOAR".into(),
                value: "https://example.test/feed".into(),
            },
            Id3v24Edit {
                frame_label: "WOAF".into(),
                value: "https://example.test/track".into(),
            },
        ];

        write_id3v24_edits(temp.path(), &edits).expect("first write");
        let first = read_audio_tags(temp.path())
            .expect("read after first write")
            .fields;
        write_id3v24_edits(temp.path(), &edits).expect("second write");
        let second = read_audio_tags(temp.path())
            .expect("read after second write")
            .fields;

        assert_eq!(
            first, second,
            "a repeated FLAC write must not duplicate a field"
        );
    }

    /// Orchestrator review, defect 2: two edits that alias to the same
    /// Vorbis key (`TDRC` and `TYER`, both `DATE`) keep both their values.
    /// A once-per-edit removal would let the second edit erase the first.
    #[test]
    fn adr_0080_two_edits_sharing_one_vorbis_key_keep_both_values() {
        let flac_bytes = include_bytes!("../docs/runbooks/fixtures/conversion.flac");
        let temp = tempfile::Builder::new()
            .suffix(".flac")
            .tempfile()
            .expect("temp flac file");
        fs::write(temp.path(), flac_bytes).expect("write flac fixture copy");
        let edits = [
            Id3v24Edit {
                frame_label: "TDRC".into(),
                value: "2024-01-01".into(),
            },
            Id3v24Edit {
                frame_label: "TYER".into(),
                value: "2024".into(),
            },
        ];

        write_id3v24_edits(temp.path(), &edits).expect("write edits");
        let tags = read_audio_tags(temp.path()).expect("read tags back");

        let dates = tags
            .fields
            .iter()
            .filter(|field| field.frame_id == "TDRC")
            .map(|field| field.value.as_str())
            .collect::<Vec<_>>();
        assert!(
            dates.contains(&"2024-01-01"),
            "the TDRC value must stay: {dates:?}"
        );
        assert!(
            dates.contains(&"2024"),
            "the TYER value must also stay: {dates:?}"
        );
        assert_eq!(
            dates.len(),
            2,
            "both values must be present, not one replacing the other: {dates:?}"
        );
    }

    /// R80-07: an MP3 file with the earlier labeled channel value and the
    /// item page mistakenly held in `WOAR` holds only the plain channel
    /// value in `WOAR`, and the item page in `WOAF`, after one write.
    #[test]
    fn adr_0080_write_moves_earlier_woar_values_to_their_frame() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"not really an mp3").expect("write file");
        let channel_url = "https://example.test/feed";
        let item_page = "https://example.test/track";
        write_id3v24_edits(
            temp.path(),
            &[
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: format!("download for free (url, forward): {channel_url}"),
                },
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: item_page.into(),
                },
            ],
        )
        .expect("seed the earlier mapping");

        write_id3v24_edits(
            temp.path(),
            &[
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: channel_url.into(),
                },
                Id3v24Edit {
                    frame_label: "WOAF".into(),
                    value: item_page.into(),
                },
            ],
        )
        .expect("write the current mapping");

        let tag = Tag::read_from_path(temp.path()).expect("read written tag");
        assert_eq!(
            woar_frame_values(&tag),
            vec![channel_url.to_string()],
            "WOAR must hold only the plain channel value"
        );
        assert_eq!(
            woaf_frame_values(&tag),
            vec![item_page.to_string()],
            "WOAF must hold only the item page"
        );
    }

    /// R80-08: a write keeps a `WOAR` value that no source supplied, and a
    /// MusicBrainz `WOAR` value, while it still moves the earlier labeled
    /// channel value and the item page to their own frame.
    #[test]
    fn adr_0080_write_keeps_foreign_and_musicbrainz_woar_values() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"not really an mp3").expect("write file");
        let foreign_url = "https://foreign.example/from-another-tool";
        let musicbrainz_value =
            "download for free (url, forward): https://musicbrainz.example/artist";
        let channel_url = "https://example.test/feed";
        let item_page = "https://example.test/track";
        write_id3v24_edits(
            temp.path(),
            &[
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: foreign_url.into(),
                },
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: musicbrainz_value.into(),
                },
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: format!("download for free (url, forward): {channel_url}"),
                },
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: item_page.into(),
                },
            ],
        )
        .expect("seed the file with three owners and the earlier mapping");

        write_id3v24_edits(
            temp.path(),
            &[
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: channel_url.into(),
                },
                Id3v24Edit {
                    frame_label: "WOAF".into(),
                    value: item_page.into(),
                },
            ],
        )
        .expect("write the current mapping");

        let tag = Tag::read_from_path(temp.path()).expect("read written tag");
        let mut expected = vec![
            foreign_url.to_string(),
            musicbrainz_value.to_string(),
            channel_url.to_string(),
        ];
        expected.sort();
        assert_eq!(
            woar_frame_values(&tag),
            expected,
            "a write keeps a foreign value and a MusicBrainz value, and \
             replaces only the values it supplies"
        );
        assert_eq!(woaf_frame_values(&tag), vec![item_page.to_string()]);
    }

    /// R82-08: a write with the RSS channel website and a MusicBrainz
    /// release-group homepage holds both plain URLs in `WOAR`. A second
    /// write gives equal frames.
    #[test]
    fn adr_0080_mb_write_keeps_channel_and_musicbrainz_homepage_in_woar() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"not really an mp3").expect("write file");
        let channel_url = "https://example.test/feed";
        let homepage_url = "https://musicbrainz.example/homepage";
        let edits = [
            Id3v24Edit {
                frame_label: "WOAR".into(),
                value: channel_url.into(),
            },
            Id3v24Edit {
                frame_label: "WOAR".into(),
                value: homepage_url.into(),
            },
        ];

        write_id3v24_edits(temp.path(), &edits).expect("first write");
        let tag = Tag::read_from_path(temp.path()).expect("read written tag");
        let mut expected = vec![channel_url.to_string(), homepage_url.to_string()];
        expected.sort();
        assert_eq!(
            woar_frame_values(&tag),
            expected,
            "WOAR must hold both the channel website and the MusicBrainz homepage"
        );

        let first = read_audio_tags(temp.path())
            .expect("read after first write")
            .fields;
        write_id3v24_edits(temp.path(), &edits).expect("second write");
        let second = read_audio_tags(temp.path())
            .expect("read after second write")
            .fields;
        assert_eq!(
            first, second,
            "a repeated write must not duplicate a WOAR frame"
        );
    }

    /// The audio bytes after each fixture tag: the header of one MPEG frame
    /// and a few data bytes. The first byte is not zero, so no reader takes
    /// it as tag padding.
    const OLD_ITUNES_AUDIO: &[u8] = &[
        0xFF, 0xFB, 0x90, 0x64, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA,
    ];

    fn syncsafe(size: usize) -> [u8; 4] {
        let size = u32::try_from(size).expect("fixture size");
        [
            u8::try_from((size >> 21) & 0x7F).expect("byte"),
            u8::try_from((size >> 14) & 0x7F).expect("byte"),
            u8::try_from((size >> 7) & 0x7F).expect("byte"),
            u8::try_from(size & 0x7F).expect("byte"),
        ]
    }

    /// An ID3v2.2 text frame with ISO-8859-1 text.
    fn v22_text_frame(id: &[u8; 3], text: &str) -> Vec<u8> {
        let size = u32::try_from(text.len() + 1).expect("frame size");
        let mut frame = id.to_vec();
        frame.extend_from_slice(&size.to_be_bytes()[1..]);
        frame.push(0);
        frame.extend_from_slice(text.as_bytes());
        frame
    }

    /// An ID3v2.4 frame with no flags.
    fn v24_frame(id: &[u8; 4], content: &[u8]) -> Vec<u8> {
        let mut frame = id.to_vec();
        frame.extend_from_slice(&syncsafe(content.len()));
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(content);
        frame
    }

    /// A file of one ID3v2 tag with `frames`, followed by the fixture audio.
    fn tagged_fixture(major_version: u8, frames: &[Vec<u8>]) -> Vec<u8> {
        let body = frames.concat();
        let mut bytes = b"ID3".to_vec();
        bytes.extend_from_slice(&[major_version, 0, 0]);
        bytes.extend_from_slice(&syncsafe(body.len()));
        bytes.extend_from_slice(&body);
        bytes.extend_from_slice(OLD_ITUNES_AUDIO);
        bytes
    }

    /// The bytes that follow the ID3v2 tag at the start of `bytes`.
    fn audio_after_tag(bytes: &[u8]) -> &[u8] {
        assert_eq!(&bytes[..3], b"ID3", "the file must start with a tag");
        let size = bytes[6..10]
            .iter()
            .fold(0_usize, |size, byte| (size << 7) | usize::from(*byte));
        &bytes[10 + size..]
    }

    fn old_itunes_edits() -> Vec<Id3v24Edit> {
        [
            ("TIT2", "Make It"),
            ("TALB", "Disco Swag - The Album"),
            ("TPE1", "The Doerfels"),
            ("TXXX:RSS Item GUID", "item-guid"),
        ]
        .into_iter()
        .map(|(frame_label, value)| Id3v24Edit {
            frame_label: frame_label.into(),
            value: value.into(),
        })
        .collect()
    }

    /// R80-4-01: a write on a file with an ID3v2.2 tag from iTunes gives an
    /// ID3v2.4 tag. The `TSP` value moves to `TSOP`, the audio bytes stay
    /// equal, and the file mode stays the same.
    #[test]
    fn adr_0080_old_itunes_frames_v22_sort_frame_becomes_tsop() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("make-it.mp3");
        let fixture = tagged_fixture(
            2,
            &[
                v22_text_frame(b"TT2", "Make It (old)"),
                v22_text_frame(b"TSP", "Doerfels"),
            ],
        );
        fs::write(&path, &fixture).expect("write fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("set mode");

        write_id3v24_edits(&path, &old_itunes_edits()).expect("write tags");

        let tag = Tag::read_from_path(&path).expect("read written tag");
        assert_eq!(tag.version(), id3::Version::Id3v24);
        assert_eq!(tag.title(), Some("Make It"));
        assert_eq!(tag.album(), Some("Disco Swag - The Album"));
        assert_eq!(tag.artist(), Some("The Doerfels"));
        assert_eq!(
            tag.get("TSOP").and_then(|frame| frame.content().text()),
            Some("Doerfels")
        );
        assert!(tag
            .extended_texts()
            .any(|text| text.description == "RSS Item GUID" && text.value == "item-guid"));
        let written = fs::read(&path).expect("read written file");
        assert_eq!(audio_after_tag(&written), OLD_ITUNES_AUDIO);
        let mode = fs::metadata(&path).expect("metadata").permissions().mode();
        assert_eq!(mode & 0o777, 0o640);
    }

    /// R80-4-03: a write that fails during the encode leaves each byte of
    /// the file unchanged and leaves no staged copy. The fixture `MLLT`
    /// frame decodes, but its field widths cannot be encoded.
    #[test]
    fn adr_0080_old_itunes_frames_failed_encode_leaves_file_unchanged() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("track.mp3");
        let mllt = [0, 1, 0, 0, 1, 0, 0, 1, 1, 1];
        let fixture = tagged_fixture(
            4,
            &[
                v24_frame(b"TIT2", b"\x03Old title"),
                v24_frame(b"MLLT", &mllt),
            ],
        );
        fs::write(&path, &fixture).expect("write fixture");

        let result = write_id3v24_edits(&path, &old_itunes_edits());

        assert!(result.is_err(), "the encode must fail: {result:?}");
        assert_eq!(fs::read(&path).expect("read file"), fixture);
        let entries = fs::read_dir(temp.path()).expect("read dir").count();
        assert_eq!(entries, 1, "a failed write must leave no staged copy");
    }

    /// R80-4-02: a write removes an ID3v2.2 frame that has no ID3v2.4 ID,
    /// and the write result names that frame. The write keeps the other
    /// old values.
    #[test]
    fn adr_0080_old_itunes_frames_unmapped_frame_is_removed_and_named() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("track.mp3");
        let fixture = tagged_fixture(
            2,
            &[
                v22_text_frame(b"TT2", "Old title"),
                v22_text_frame(b"TCP", "1"),
                v22_text_frame(b"XYZ", "unknown"),
            ],
        );
        fs::write(&path, &fixture).expect("write fixture");

        let result = write_id3v24_edits(&path, &old_itunes_edits()).expect("write tags");

        assert_eq!(result.applied, 4);
        assert_eq!(result.removed_frames, vec!["XYZ".to_owned()]);
        let tag = Tag::read_from_path(&path).expect("read written tag");
        assert!(tag.frames().all(|frame| frame.id() != "XYZ"));
        assert_eq!(
            tag.get("TCMP").and_then(|frame| frame.content().text()),
            Some("1")
        );
        let written = fs::read(&path).expect("read written file");
        assert_eq!(audio_after_tag(&written), OLD_ITUNES_AUDIO);
    }

    /// R80-4-05: on a file with an ID3v2.3 or ID3v2.4 tag, the write gives
    /// the same bytes as the earlier write in place.
    #[test]
    fn adr_0080_old_itunes_frames_v23_and_v24_writes_are_unchanged() {
        for major_version in [3, 4] {
            let temp = tempfile::tempdir().expect("temp dir");
            let path = temp.path().join("track.mp3");
            let expected_path = temp.path().join("expected.mp3");
            let text = |value: &str| {
                let mut content = vec![if major_version == 4 { 3 } else { 0 }];
                content.extend_from_slice(value.as_bytes());
                content
            };
            let frame = |id: &[u8; 4], content: &[u8]| {
                let mut frame = v24_frame(id, content);
                if major_version == 3 {
                    let size = u32::try_from(content.len()).expect("frame size");
                    frame[4..8].copy_from_slice(&size.to_be_bytes());
                }
                frame
            };
            let fixture = tagged_fixture(
                major_version,
                &[
                    frame(b"TIT2", &text("Old title")),
                    frame(b"TPE2", &text("Other tool artist")),
                    frame(b"TSOP", &text("Doerfels")),
                ],
            );
            fs::write(&path, &fixture).expect("write fixture");
            fs::write(&expected_path, &fixture).expect("write expected fixture");
            let edits = old_itunes_edits();

            let result = write_id3v24_edits(&path, &edits).expect("write tags");

            let mut tag = Tag::read_from_path(&expected_path).expect("read fixture tag");
            for edit in &edits {
                tag.add_frame(super::id3v24_edit_frame(edit).expect("edit frame"));
            }
            tag.write_to_path(&expected_path, id3::Version::Id3v24)
                .expect("write in place");
            assert_eq!(result.applied, edits.len());
            assert!(result.removed_frames.is_empty());
            assert_eq!(
                fs::read(&path).expect("read written file"),
                fs::read(&expected_path).expect("read expected file"),
                "ID3v2.{major_version}"
            );
        }
    }
}
