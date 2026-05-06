//! Test helpers for Discover metadata grid row construction.

#![cfg(test)]
#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

use crate::audio_tags::Id3Field;
use crate::metadata::{
    id3_frame_base, id3_frame_group_key, pending_id3_target_key, AlignedCompareRow,
    MetadataGridRow, MetadataGroupRow, TagCompareResult, ID3V24_FRAME_IDS,
};
use crate::track_compare::ComparisonStatus;
use crate::view_models::track_metadata_grid::TrackMetadataGridVm;

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn metadata_group_row(
    label: impl Into<String>,
    key: Option<&str>,
    expanded: bool,
    unused_count: usize,
) -> MetadataGridRow {
    MetadataGridRow::Group(MetadataGroupRow {
        key: key.map(str::to_string),
        label: label.into(),
        expanded,
        unused_count,
    })
}

#[cfg(test)]
pub(crate) fn metadata_data_row(row: AlignedCompareRow) -> MetadataGridRow {
    MetadataGridRow::Data(row)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn id3_unused_frame_row(frame_id: &str) -> MetadataGridRow {
    metadata_data_row(AlignedCompareRow {
        row_id: TrackMetadataGridVm::unused_id3_frame_row_id(frame_id),
        field: TrackMetadataGridVm::id3_field_display_label(frame_id),
        rss_value: None,
        id3_value: None,
        id3_frame: Some(frame_id.into()),
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingBoth,
        musicbrainz_status: ComparisonStatus::MissingBoth,
    })
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn used_id3_field_row(field: &Id3Field) -> MetadataGridRow {
    metadata_data_row(AlignedCompareRow {
        row_id: TrackMetadataGridVm::used_id3_field_row_id(&field.frame_id),
        field: TrackMetadataGridVm::id3_field_display_label(&field.frame_id),
        rss_value: None,
        id3_value: Some(field.value.clone()),
        id3_frame: Some(field.frame_id.clone()),
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingSource,
        musicbrainz_status: ComparisonStatus::MissingBoth,
    })
}

#[cfg(test)]
pub(crate) fn unused_id3v24_frames_for_group(
    result: &TagCompareResult,
    group_key: &str,
) -> Vec<&'static str> {
    ID3V24_FRAME_IDS
        .iter()
        .copied()
        .filter(|frame_id| id3_frame_group_key(frame_id) == group_key)
        .filter(|frame_id| {
            !result
                .id3_fields
                .iter()
                .any(|field| id3_frame_base(&field.frame_id) == *frame_id)
        })
        .collect()
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn used_id3_fields_for_group<'a>(
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
#[cfg(test)]
pub(crate) fn id3_frame_hint(field: &str) -> Option<&'static str> {
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
        "Website" => Some("WOAR"),
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

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn id3_frame_is_summarized(frame_id: &str) -> bool {
    matches!(frame_id, "TIT2" | "TPE1" | "TALB" | "TRCK")
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn format_track_slash_total(track: Option<&str>, total: Option<&str>) -> Option<String> {
    match (track, total) {
        (Some(t), Some(tot)) => Some(format!("{t}/{tot}")),
        (Some(t), None) => Some(t.to_string()),
        (None, Some(tot)) => Some(format!("/{tot}")),
        (None, None) => None,
    }
}
