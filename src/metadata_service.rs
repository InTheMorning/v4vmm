use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rusqlite::Connection;

use crate::api::{PaymentRoute, Track};
use crate::audio_tags::{AudioTags, Id3v24Edit};
use crate::db;
use crate::metadata::{
    auto_populated_pending_id3_edits, expand_woar_metadata_rows, feed_nostr,
    pending_id3_edits_for_apply, summarize_value_routes, track_metadata_rows, track_nostr,
    MetadataColumn, PendingId3Edit, TrackContext, MUSICINDEX_VALUE_ROUTES_FRAME,
};
use crate::musicbrainz::LookupMetadata;
use crate::rss::{validate_nostr_identity, IdentityValidation, NostrIdentity};

/// The frame that holds the one resolved Nostr key (ADR 0080 Decision 1).
const RSS_NOSTR_HANDLE_FRAME: &str = "TXXX:RSS Nostr Handle";

pub fn id3_edits_for_track_context(track_context: &TrackContext) -> Vec<Id3v24Edit> {
    let rows = expand_woar_metadata_rows(track_metadata_rows(track_context, None, false));
    let mut pending =
        auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new(), None);
    apply_resolved_nostr_key(&mut pending, track_context);
    pending_id3_edits_for_apply(&pending)
}

/// ADR 0080 Decisions 1 and 7: a file holds one Nostr key. The item's own
/// key wins when it passes NIP-19 validation as an `npub`. The channel's key
/// wins when the item has none, or when its key fails validation. Neither
/// key valid leaves the file with no Nostr frame.
///
/// The compare grid keeps its own "Nostr handle" and "RSS feed nostr handle"
/// rows apart (ADR 0075 Decision B). This function only resolves the value
/// that a write puts in the file.
fn apply_resolved_nostr_key(
    pending: &mut BTreeMap<String, PendingId3Edit>,
    track_context: &TrackContext,
) {
    pending.retain(|_, edit| edit.frame != RSS_NOSTR_HANDLE_FRAME);
    let Some(value) = resolved_nostr_key(track_context) else {
        return;
    };
    pending.insert(
        "adr-0080:resolved-nostr-handle".to_owned(),
        PendingId3Edit {
            field: "Nostr handle".to_owned(),
            frame: RSS_NOSTR_HANDLE_FRAME.to_owned(),
            value,
            source: MetadataColumn::Rss,
        },
    );
}

/// The item's own Nostr key, when it is a valid `npub`. Otherwise the
/// channel's key, under the same rule. `None` when neither key validates.
fn resolved_nostr_key(track_context: &TrackContext) -> Option<String> {
    let item_key = track_nostr(&track_context.track);
    valid_npub(item_key.as_deref()).or_else(|| {
        let channel_key = track_context.feed.as_ref().and_then(feed_nostr);
        valid_npub(channel_key.as_deref())
    })
}

/// `Some` when `candidate` decodes as a valid NIP-19 `npub`. A `nprofile`
/// value and an invalid value are not accepted (ADR 0080 Decision 7).
fn valid_npub(candidate: Option<&str>) -> Option<String> {
    let candidate = candidate?;
    match validate_nostr_identity(candidate) {
        IdentityValidation::Valid(NostrIdentity::PublicKey { .. }) => Some(candidate.to_owned()),
        _ => None,
    }
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

    /// A real, validly encoded `npub` for the track (ADR 0080 Decision 7).
    const NPUB_TRACK: &str = "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6";
    /// A real, validly encoded `npub` for the feed, distinct from the
    /// track's, so a test can tell which one a resolved edit carried.
    const NPUB_FEED: &str = "npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck";

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
                    value: Some(NPUB_FEED.into()),
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
                    value: Some(NPUB_TRACK.into()),
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
                    value: Some(NPUB_FEED.into()),
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

    /// R22-06 / R80-11 (ADR 0075 packet 022, updated by ADR 0080 packet
    /// 001): a track with no own website, Nostr key or description tags the
    /// channel's values in their own frames. The channel website goes to
    /// `WOAR` (Decision 2), the channel description goes to
    /// `COMM:MusicIndex Album Description` (Decision 5) because the item
    /// has none, and the channel's Nostr key resolves into
    /// `TXXX:RSS Nostr Handle` (Decision 1) because the item has none. This
    /// test holds the expected edits as fixed values. It also proves ADR
    /// 0011: the feed and track GUIDs map to their own `TXXX` frames.
    #[test]
    fn adr_0075_track_header_r22_06_tag_edits_stay_equal_without_own_identity() {
        let context = track_context_without_own_identity();
        let edits = id3_edits_for_track_context(&context);

        assert_eq!(
            edits,
            vec![
                edit("COMM:MusicIndex Album Description", "Feed description"),
                edit("TALB", "Feed Title"),
                edit("TIT2", "Song"),
                edit("TXXX:MusicIndex Feed Guid", "feed-guid"),
                edit("TXXX:MusicIndex Track Guid", "track-guid"),
                edit("TXXX:RSS Nostr Handle", NPUB_FEED),
                edit("WOAR", "https://example.test/feed"),
            ],
            "ADR 0011 feed/track GUID frames and ADR 0075/0080 track header tags must match exactly"
        );
    }

    /// R22-06 / R80-11: a track with its own website, Nostr key and
    /// description tags its own values, apart from the feed's, and each
    /// frame now follows ADR 0080: the item page goes to `WOAF` and the
    /// channel website to `WOAR` (Decision 2), the item's own Nostr key
    /// wins over the channel's (Decision 1), and the item's own description
    /// stays in `COMM:MusicIndex Description` with no album row (Decision
    /// 5). It also proves ADR 0011: the feed and track GUIDs map to their
    /// own `TXXX` frames.
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
                edit("TXXX:RSS Nostr Handle", NPUB_TRACK),
                edit("WOAF", "https://example.test/track"),
                edit("WOAR", "https://example.test/feed"),
            ],
            "ADR 0011 feed/track GUID frames and ADR 0075/0080 track header tags must match exactly"
        );
    }

    /// R80-01: a track with its own valid Nostr key holds that key. Without
    /// one, the edits hold the valid channel key.
    #[test]
    fn adr_0080_nostr_edit_prefers_the_item_key_then_the_channel_key() {
        let with_own_key = id3_edits_for_track_context(&track_context_with_own_identity());
        assert!(with_own_key.contains(&edit("TXXX:RSS Nostr Handle", NPUB_TRACK)));

        let without_own_key = id3_edits_for_track_context(&track_context_without_own_identity());
        assert!(without_own_key.contains(&edit("TXXX:RSS Nostr Handle", NPUB_FEED)));
    }

    /// R80-02: an invalid item key gives the valid channel key. Two invalid
    /// keys give no Nostr edit.
    #[test]
    fn adr_0080_nostr_edit_falls_back_past_an_invalid_key() {
        let mut context = track_context_with_own_identity();
        context.track.source_ids = Some(vec![SourceEntityId {
            scheme: Some("nostr_npub".into()),
            value: Some("npub1notavalidkey".into()),
            ..Default::default()
        }]);
        let edits = id3_edits_for_track_context(&context);
        assert!(
            edits.contains(&edit("TXXX:RSS Nostr Handle", NPUB_FEED)),
            "an invalid item key must give the valid channel key"
        );

        context.feed.as_mut().expect("feed").source_ids = Some(vec![SourceEntityId {
            scheme: Some("nostr_npub".into()),
            value: Some("npub1bbbbbb".into()),
            ..Default::default()
        }]);
        let edits = id3_edits_for_track_context(&context);
        assert!(
            !edits
                .iter()
                .any(|edit| edit.frame_label == "TXXX:RSS Nostr Handle"),
            "two invalid keys must give no Nostr edit"
        );
    }

    /// R80-03: the item page goes to `WOAF` and the channel website goes to
    /// `WOAR`. Neither value lands in the other frame.
    #[test]
    fn adr_0080_website_edits_stay_apart_by_owner() {
        let edits = id3_edits_for_track_context(&track_context_with_own_identity());
        assert!(edits.contains(&edit("WOAF", "https://example.test/track")));
        assert!(edits.contains(&edit("WOAR", "https://example.test/feed")));
        assert!(!edits.contains(&edit("WOAR", "https://example.test/track")));
        assert!(!edits.contains(&edit("WOAF", "https://example.test/feed")));
    }

    /// R80-04: a URL edit holds a plain URL, with no label text. A value
    /// that does not parse as a URL writes no edit at all.
    #[test]
    fn adr_0080_website_edit_is_a_plain_url_or_absent() {
        let edits = id3_edits_for_track_context(&track_context_with_own_identity());
        let woar = edits
            .iter()
            .find(|edit| edit.frame_label == "WOAR")
            .expect("WOAR edit");
        assert_eq!(woar.value, "https://example.test/feed");

        let mut context = track_context_with_own_identity();
        context.track.source_links = Some(vec![SourceEntityLink {
            link_type: Some("website".into()),
            url: Some("not a url".into()),
            ..Default::default()
        }]);
        let edits = id3_edits_for_track_context(&context);
        assert!(
            !edits.iter().any(|edit| edit.frame_label == "WOAF"),
            "a value that does not parse as a URL must write no WOAF edit"
        );
    }

    /// R80-05: a track without its own description gives
    /// `COMM:MusicIndex Album Description` with the channel description, and
    /// no `COMM:MusicIndex Description`.
    #[test]
    fn adr_0080_description_falls_back_to_the_album_row_only() {
        let edits = id3_edits_for_track_context(&track_context_without_own_identity());
        assert!(edits.contains(&edit(
            "COMM:MusicIndex Album Description",
            "Feed description"
        )));
        assert!(!edits
            .iter()
            .any(|edit| edit.frame_label == "COMM:MusicIndex Description"));
    }

    /// R80-09: a round trip. Fresh edits, written to a file and read back,
    /// carry the same value under the same frame label. A DB-backed scan
    /// (`application::queries::tag_update::changed_frames`) compares a file
    /// the same way, so it reports no difference for a file this app wrote.
    #[test]
    fn adr_0080_round_trip_edits_read_back_unchanged() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(temp.path(), b"not really an mp3").expect("write file");
        let edits = id3_edits_for_track_context(&track_context_with_own_identity());

        crate::audio_tags::write_id3v24_edits(temp.path(), &edits).expect("write edits");
        let tags = crate::audio_tags::read_audio_tags(temp.path()).expect("read tags back");

        for expected in &edits {
            let found = tags.fields.iter().any(|field| {
                field.frame_id == expected.frame_label && field.value == expected.value
            });
            assert!(
                found,
                "frame {} must read back with the value the writer produced",
                expected.frame_label
            );
        }
    }

    /// R49-04 (ADR 0075 packet 049): a track with its own date gives a
    /// `TDRC` edit with that date.
    #[test]
    fn adr_0075_release_date_r49_04_own_date_writes_tdrc() {
        let mut context = track_context_without_own_identity();
        context.track.pub_date = Some(1_704_067_200);
        let edits = id3_edits_for_track_context(&context);
        assert!(edits.contains(&edit("TDRC", "2024-01-01")));
    }

    /// R49-05 (ADR 0075 packet 049): a track without its own date gets no
    /// `TDRC` edit from a feed release date or an oldest item date.
    #[test]
    fn adr_0075_release_date_r49_05_no_own_date_writes_no_tdrc() {
        let mut context = track_context_without_own_identity();
        context.feed.as_mut().expect("feed").release_date = Some(1_672_531_200);
        context.feed.as_mut().expect("feed").oldest_item_at = Some(1_640_995_200);
        let edits = id3_edits_for_track_context(&context);
        assert!(
            !edits.iter().any(|edit| edit.frame_label == "TDRC"),
            "a track with no own date must write no TDRC edit"
        );
    }
}
