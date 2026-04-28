use std::collections::{BTreeMap, BTreeSet};

use crate::api::Track;
use crate::audio_tags::{AudioTags, Id3v24Edit};
use crate::metadata::{
    auto_populated_pending_id3_edits, expand_woar_metadata_rows, pending_id3_edits_for_apply,
    track_metadata_rows, TrackContext,
};
use crate::musicbrainz::LookupMetadata;

pub fn id3_edits_for_track_context(track_context: &TrackContext) -> Vec<Id3v24Edit> {
    let rows = expand_woar_metadata_rows(track_metadata_rows(track_context, None, false));
    let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new(), None);
    pending_id3_edits_for_apply(&pending)
}

pub fn musicbrainz_lookup_metadata(track: &Track, tags: &AudioTags) -> LookupMetadata {
    LookupMetadata {
        title: tags
            .title
            .clone()
            .or_else(|| track.title.clone())
            .or_else(|| track.name.clone()),
        artist: tags.artist.clone().or_else(|| track.track_artist.clone()),
        album: tags.album.clone().or_else(|| track.feed_title.clone()),
        track_number: tags
            .track_number
            .clone()
            .or_else(|| track.track_number.map(|number| number.to_string())),
        total_tracks: None,
        duration_secs: track.duration_secs.map(i64::from),
        isrc: tags
            .custom
            .get("ISRC")
            .cloned()
            .or_else(|| tags.custom.get("isrc").cloned()),
    }
}
