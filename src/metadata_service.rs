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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Feed, SourceEntityId, SourceEntityLink};

    /// A track with no website, no Nostr key and no description of its own.
    /// Its feed has all three.
    fn track_context_without_own_identity() -> TrackContext {
        TrackContext {
            track: Track {
                track_guid: Some("track-guid".into()),
                feed_guid: Some("feed-guid".into()),
                title: Some("Song".into()),
                ..Default::default()
            },
            feed: Some(Feed {
                feed_guid: Some("feed-guid".into()),
                title: Some("Feed Title".into()),
                description: Some("Feed description".into()),
                source_links: Some(vec![SourceEntityLink {
                    link_type: Some("website".into()),
                    url: Some("https://example.test/feed".into()),
                    ..Default::default()
                }]),
                source_ids: Some(vec![SourceEntityId {
                    scheme: Some("nostr_npub".into()),
                    value: Some("npub1feed".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            rss_observation: None,
            observation_receipts: Vec::new(),
            provider_state: Default::default(),
        }
    }

    /// A track with its own website, Nostr key and description, apart from
    /// its feed's own distinct values.
    fn track_context_with_own_identity() -> TrackContext {
        TrackContext {
            track: Track {
                track_guid: Some("track-guid".into()),
                feed_guid: Some("feed-guid".into()),
                title: Some("Song".into()),
                description: Some("Track description".into()),
                source_links: Some(vec![SourceEntityLink {
                    link_type: Some("website".into()),
                    url: Some("https://example.test/track".into()),
                    ..Default::default()
                }]),
                source_ids: Some(vec![SourceEntityId {
                    scheme: Some("nostr_npub".into()),
                    value: Some("npub1track".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            feed: Some(Feed {
                feed_guid: Some("feed-guid".into()),
                title: Some("Feed Title".into()),
                description: Some("Feed description".into()),
                source_links: Some(vec![SourceEntityLink {
                    link_type: Some("website".into()),
                    url: Some("https://example.test/feed".into()),
                    ..Default::default()
                }]),
                source_ids: Some(vec![SourceEntityId {
                    scheme: Some("nostr_npub".into()),
                    value: Some("npub1feed".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            rss_observation: None,
            observation_receipts: Vec::new(),
            provider_state: Default::default(),
        }
    }

    fn edit(frame_label: &str, value: &str) -> Id3v24Edit {
        Id3v24Edit {
            frame_label: frame_label.to_owned(),
            value: value.to_owned(),
        }
    }

    /// R22-06 (ADR 0075 packet 022): `id3_edits_for_track_context` gives
    /// equal tag edits before and after this packet. A track with no own
    /// website, Nostr key or description still tags the feed's WOAR,
    /// `TXXX:RSS Nostr Handle` and `COMM:MusicIndex Description` frames,
    /// because the "RSS feed website"/"RSS feed nostr handle" rows and the
    /// "Description" row's feed fallback map to the same frames as the
    /// track's own rows (`id3_frame_hint`). This test holds the expected
    /// edits as fixed values.
    #[test]
    fn adr_0075_track_header_r22_06_tag_edits_stay_equal_without_own_identity() {
        let context = track_context_without_own_identity();
        let edits = id3_edits_for_track_context(&context);

        assert_eq!(
            edits,
            vec![
                edit("COMM:MusicIndex Description", "Feed description"),
                edit("TALB", "Feed Title"),
                edit("TIT2", "Song"),
                edit("TXXX:MusicIndex Feed Guid", "feed-guid"),
                edit("TXXX:MusicIndex Track Guid", "track-guid"),
                edit("TXXX:RSS Nostr Handle", "npub1feed"),
                edit(
                    "WOAR",
                    "download for free (url, forward): https://example.test/feed"
                ),
            ]
        );
    }

    /// R22-06: a track with its own website, Nostr key and description
    /// tags its own values, apart from the feed's, and the resulting edits
    /// stay equal before and after this packet.
    #[test]
    fn adr_0075_track_header_r22_06_tag_edits_stay_equal_with_own_identity() {
        let context = track_context_with_own_identity();
        let edits = id3_edits_for_track_context(&context);

        assert_eq!(
            edits,
            vec![
                edit("COMM:MusicIndex Description", "Track description"),
                edit("TALB", "Feed Title"),
                edit("TIT2", "Song"),
                edit("TXXX:MusicIndex Feed Guid", "feed-guid"),
                edit("TXXX:MusicIndex Track Guid", "track-guid"),
                edit("TXXX:RSS Nostr Handle", "npub1feed"),
                edit(
                    "WOAR",
                    "download for free (url, forward): https://example.test/feed"
                ),
                edit(
                    "WOAR",
                    "download for free (url, forward): https://example.test/track"
                ),
            ]
        );
    }
}
