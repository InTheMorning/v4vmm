use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rusqlite::Connection;

use crate::api::{PaymentRoute, Track};
use crate::audio_tags::{AudioTags, Id3v24Edit};
use crate::db;
use crate::metadata::{
    auto_populated_pending_id3_edits, expand_woar_metadata_rows, pending_id3_edits_for_apply,
    summarize_value_routes, track_metadata_rows, TrackContext, MUSICINDEX_VALUE_ROUTES_FRAME,
};
use crate::musicbrainz::LookupMetadata;

pub fn id3_edits_for_track_context(track_context: &TrackContext) -> Vec<Id3v24Edit> {
    let rows = expand_woar_metadata_rows(track_metadata_rows(track_context, None, false));
    let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new(), None);
    pending_id3_edits_for_apply(&pending)
}

/// How a tag write treats the payment route frame (ADR 0076 Decision 9).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteFrameWrite {
    /// The write always sets the frame to the stored route. A download, the
    /// route repair and a feed update write the frame this way.
    Always,
    /// The write changes the frame only when the edit list already has a
    /// route frame. An operator who applies selected tag edits writes the
    /// frame this way, so an edit that the operator did not select is not
    /// added.
    WhenSelected,
}

/// ADR 0076 Decision 9: each write of the payment route frame uses the route
/// stored in the database. The function removes each route frame edit from
/// `edits` and then adds one edit with the stored route.
///
/// When the database has no route for the track, the `MusicIndex` route of
/// the caller goes into the database first. The edit then uses the stored
/// value. An empty or missing stored route gives no route frame edit.
///
/// # Errors
///
/// Returns an error when the stored route cannot be read or written.
pub fn with_stored_route_frame(
    conn: &Connection,
    track_id: i64,
    musicindex_routes: Option<&[PaymentRoute]>,
    mut edits: Vec<Id3v24Edit>,
    write: RouteFrameWrite,
) -> Result<Vec<Id3v24Edit>> {
    let selected = edits
        .iter()
        .any(|edit| edit.frame_label == MUSICINDEX_VALUE_ROUTES_FRAME);
    if write == RouteFrameWrite::WhenSelected && !selected {
        return Ok(edits);
    }
    edits.retain(|edit| edit.frame_label != MUSICINDEX_VALUE_ROUTES_FRAME);
    let stored = db::payment_routes::route_for_write(conn, track_id, musicindex_routes)?;
    if let Some(value) = stored.as_deref().and_then(summarize_value_routes) {
        edits.push(Id3v24Edit {
            frame_label: MUSICINDEX_VALUE_ROUTES_FRAME.to_owned(),
            value,
        });
    }
    Ok(edits)
}

pub fn musicbrainz_lookup_metadata(track: &Track, tags: &AudioTags) -> LookupMetadata {
    LookupMetadata {
        title: tags.title.clone().or_else(|| track.title.clone()),
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
