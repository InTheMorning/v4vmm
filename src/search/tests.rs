use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use super::{
    aligned_compare_rows, aligned_id3_frame_ids, artist_rows_from_result_rows,
    auto_populated_pending_id3_edits, expand_woar_metadata_rows, feed_rss_url,
    format_drag_value_for_id3v24, id3_frame_group_key, id3_frame_version, merge_track_play_fields,
    metadata_data_row, metadata_drag_value, metadata_field_group_key, musicbrainz_remainder_rows,
    pending_id3_conflict_descriptions, pending_id3_edits_for_apply, pending_id3_target_key,
    persist_musicindex_artist_facts, search_result_type_is_visible, should_show_inspector_back,
    track_metadata_rows, unused_id3v24_frames_for_group, AlignedCompareRow, Artist, EntityDetail,
    Feed, Id3FrameVersion, MetadataColumn, MetadataGridRow, PendingId3Edit, ResultRow, SearchBatch,
    TagCompareResult, Track, TrackContext, ID3V24_FRAME_GROUPS, ID3V24_FRAME_IDS,
};
use crate::api::{SourceEnclosure, SourceEntityId, SourceEntityLink};
use crate::audio_tags::{id3v24_edit_label_is_writable, Id3Field};
use crate::db;
use crate::metadata::{
    compare_id3_field_values, contributor_id3_rows, display_metadata_value,
    musicindex_contributors_id3_value,
};
use crate::musicbrainz::MusicBrainzCandidate;
use crate::track_compare::{ComparisonRow, ComparisonStatus};
use crate::view_models::track_metadata_grid::TrackMetadataGridVm;

#[test]
fn discover_back_button_is_visible_for_any_open_inspector() {
    assert!(
        !should_show_inspector_back(0),
        "empty inspector stack should not show Back"
    );
    assert!(
        should_show_inspector_back(1),
        "first opened feed, track, or publisher should show Back"
    );
    assert!(
        should_show_inspector_back(2),
        "nested inspector frames should keep showing Back"
    );
}

#[test]
fn search_results_are_limited_to_artist_feed_and_track() {
    assert!(
        search_result_type_is_visible("artist"),
        "artist results should remain searchable"
    );
    assert!(
        search_result_type_is_visible("feed"),
        "feed results should remain searchable"
    );
    assert!(
        search_result_type_is_visible("track"),
        "track results should remain searchable"
    );
    assert!(
        !search_result_type_is_visible("publisher"),
        "publisher results should only be opened from feed or track links"
    );
}

#[test]
fn search_batch_persists_explicit_musicindex_artist_facts() -> anyhow::Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    db::init_schema(&conn)?;
    db::migrate_schema(&conn)?;
    let conn = Arc::new(Mutex::new(conn));
    let batch = SearchBatch {
        rows: vec![
            ResultRow::new(
                "artist",
                "Alice",
                Some(EntityDetail::Artist(Artist {
                    artist_id: Some("artist-123".into()),
                    name: Some("Alice".into()),
                    image_url: Some("https://example.test/alice.jpg".into()),
                    url: Some("https://example.test/alice".into()),
                    ..Artist::default()
                })),
            ),
            ResultRow::new(
                "artist",
                "Name Only",
                Some(EntityDetail::Artist(Artist {
                    name: Some("Name Only".into()),
                    ..Artist::default()
                })),
            ),
        ],
        has_more: false,
        cursor: None,
    };

    persist_musicindex_artist_facts(&conn, &batch)?;

    let db = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
    let row = db::artist_source_fact(&db, "musicindex", "artist-123")?
        .expect("explicit artist id should be persisted");
    assert_eq!(row.name.as_deref(), Some("Alice"));
    assert_eq!(
        row.image_url.as_deref(),
        Some("https://example.test/alice.jpg")
    );
    assert_eq!(
        row.website_url.as_deref(),
        Some("https://example.test/alice")
    );
    assert_eq!(
        db::artist_source_fact(&db, "musicindex", "Name Only")?,
        None,
        "synthetic/name-only artist should not be persisted"
    );

    Ok(())
}

#[test]
fn feed_and_track_action_urls_skip_empty_values() {
    let feed = Feed {
        feed_url: Some(" https://example.test/feed.xml ".into()),
        ..Feed::default()
    };
    assert_eq!(
        feed_rss_url(&feed).as_deref(),
        Some("https://example.test/feed.xml")
    );

    let direct_track = Track {
        enclosure_url: Some(" https://example.test/audio.mp3 ".into()),
        ..Track::default()
    };
    assert_eq!(
        crate::view_models::track::TrackVm::new(&direct_track)
            .play_url()
            .as_deref(),
        Some("https://example.test/audio.mp3")
    );

    let source_track = Track {
        enclosure_url: Some(" ".into()),
        source_enclosures: Some(vec![
            SourceEnclosure {
                url: Some("https://example.test/alternate.mp3".into()),
                ..SourceEnclosure::default()
            },
            SourceEnclosure {
                url: Some("https://example.test/primary.mp3".into()),
                is_primary: Some(true),
                ..SourceEnclosure::default()
            },
        ]),
        ..Track::default()
    };
    assert_eq!(
        crate::view_models::track::TrackVm::new(&source_track)
            .play_url()
            .as_deref(),
        Some("https://example.test/primary.mp3")
    );
}

#[test]
fn artist_rows_are_derived_from_feed_and_track_details() {
    let rows = vec![
        ResultRow {
            entity_type: "track".into(),
            entity_id: "track-1".into(),
            detail: Some(EntityDetail::Track(Track {
                track_artist: Some("The Doerfels".into()),
                release_artist: Some("The Doerfels".into()),
                image_url: Some("https://example.test/track.png".into()),
                ..Track::default()
            })),
        },
        ResultRow {
            entity_type: "feed".into(),
            entity_id: "feed-1".into(),
            detail: Some(EntityDetail::Feed(Feed {
                release_artist: Some("The Doerfels".into()),
                image_url: Some("https://example.test/feed.png".into()),
                ..Feed::default()
            })),
        },
        ResultRow {
            entity_type: "artist".into(),
            entity_id: "other".into(),
            detail: Some(EntityDetail::Artist(Artist {
                name: Some("Other Artist".into()),
                ..Artist::default()
            })),
        },
    ];

    let artist_rows = artist_rows_from_result_rows(&rows, Some("doerfels"));

    assert_eq!(artist_rows.len(), 1);
    assert_eq!(artist_rows[0].entity_type, "artist");
    assert_eq!(artist_rows[0].entity_id, "The Doerfels");
    let Some(EntityDetail::Artist(artist)) = &artist_rows[0].detail else {
        panic!("expected artist detail");
    };
    assert_eq!(artist.track_count, Some(1));
    assert_eq!(artist.feed_count, Some(1));
    assert_eq!(
        artist.image_url.as_deref(),
        Some("https://example.test/track.png")
    );
}

#[test]
fn feed_track_play_hydration_merges_only_missing_audio_fields() {
    let mut track = Track {
        enclosure_url: Some(" ".into()),
        title: Some("Local title".into()),
        ..Track::default()
    };
    let hydrated = Track {
        enclosure_url: Some("https://example.test/audio.mp3".into()),
        enclosure_type: Some("audio/mpeg".into()),
        enclosure_bytes: Some(123),
        source_enclosures: Some(vec![SourceEnclosure {
            url: Some("https://example.test/source.mp3".into()),
            is_primary: Some(true),
            ..SourceEnclosure::default()
        }]),
        title: Some("Hydrated title".into()),
        ..Track::default()
    };

    merge_track_play_fields(&mut track, hydrated);

    assert_eq!(
        track.enclosure_url.as_deref(),
        Some("https://example.test/audio.mp3")
    );
    assert_eq!(track.enclosure_type.as_deref(), Some("audio/mpeg"));
    assert_eq!(track.enclosure_bytes, Some(123));
    assert_eq!(track.source_enclosures.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        track.title.as_deref(),
        Some("Local title"),
        "hydrating play fields should not replace displayed feed row metadata"
    );
}

#[test]
fn id3v24_frame_registry_covers_83_grouped_frames() {
    assert_eq!(
        ID3V24_FRAME_IDS.len(),
        83,
        "expected the complete ID3v2.4 frame registry"
    );
    for frame_id in ID3V24_FRAME_IDS {
        let group = id3_frame_group_key(frame_id);
        assert!(
            ID3V24_FRAME_GROUPS.iter().any(|(key, _)| *key == group),
            "frame {frame_id} should have a visible group"
        );
    }

    let grouped_count: usize = ID3V24_FRAME_GROUPS
        .iter()
        .map(|(group_key, _)| {
            ID3V24_FRAME_IDS
                .iter()
                .filter(|frame_id| id3_frame_group_key(frame_id) == *group_key)
                .count()
        })
        .sum();
    assert_eq!(
        grouped_count, 83,
        "every ID3v2.4 frame should map into the proposed HTML table groups"
    );

    let expected_counts = [
        ("identification-release-structure", 9),
        ("people-credits", 11),
        ("descriptive-technical-rights-text", 26),
        ("url-link-frames", 9),
        ("lyrics-comments-artwork-user-facing-content", 8),
        ("identity-linking-private-registration", 7),
        ("timing-seeking-audio-analysis-playback-control", 10),
        ("music-disc-acquisition-commerce", 3),
    ];
    for (group_key, expected_count) in expected_counts {
        let actual_count = ID3V24_FRAME_IDS
            .iter()
            .filter(|frame_id| id3_frame_group_key(frame_id) == group_key)
            .count();
        assert_eq!(
            actual_count, expected_count,
            "group {group_key} should match the proposed HTML table count"
        );
    }
}

#[test]
fn unused_id3v24_frames_are_grouped_and_exclude_present_frames() {
    let result = TagCompareResult {
        path: String::new(),
        rows: Vec::new(),
        file_image: None,
        contributors: Vec::new(),
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![
            Id3Field {
                frame_id: "TIT2".into(),
                value: "Title".into(),
            },
            Id3Field {
                frame_id: "APIC".into(),
                value: "cover".into(),
            },
        ],
    };

    let identification_unused =
        unused_id3v24_frames_for_group(&result, "identification-release-structure");
    let content_unused =
        unused_id3v24_frames_for_group(&result, "lyrics-comments-artwork-user-facing-content");

    assert!(
        !identification_unused.contains(&"TIT2"),
        "present title frame should not be listed"
    );
    assert!(
        !content_unused.contains(&"APIC"),
        "present artwork frame should not be listed"
    );
    assert!(
        identification_unused.contains(&"TALB"),
        "absent album frame should remain available"
    );
}

#[test]
fn descriptor_id3_rows_are_suppressed_when_semantic_row_exists() {
    let result = TagCompareResult {
        path: String::new(),
        rows: Vec::new(),
        file_image: None,
        contributors: Vec::new(),
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![Id3Field {
            frame_id: "TXXX:MusicIndex Value Routes".into(),
            value: r#"[{"recipient_name":"Alice","split":1.0}]"#.into(),
        }],
    };
    let mut grouped = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
    grouped.insert(
        "music-disc-acquisition-commerce",
        vec![metadata_data_row(test_compare_row(
            "Value Routes",
            Some(r#"[{"recipient_name":"Alice","split":1.0}]"#),
            Some(r#"[{"recipient_name":"Alice","split":1.0}]"#),
            Some("TXXX:MusicIndex Value Routes"),
            None,
        ))],
    );

    let aligned = aligned_id3_frame_ids(&result, &grouped);
    let used =
        super::used_id3_fields_for_group(&result, "descriptive-technical-rights-text", &aligned);

    assert!(
        used.is_empty(),
        "descriptor-specific TXXX rows should not also appear as raw ID3 rows"
    );
}

#[test]
fn tempo_aliases_are_displayed_as_one_metadata_row() {
    let result = TagCompareResult {
        path: String::new(),
        rows: Vec::new(),
        file_image: None,
        contributors: Vec::new(),
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![
            Id3Field {
                frame_id: "TBPM".into(),
                value: "100.0".into(),
            },
            Id3Field {
                frame_id: "TXXX:IBPM".into(),
                value: "100.0".into(),
            },
            Id3Field {
                frame_id: "TXXX:tempo".into(),
                value: "100.0".into(),
            },
            Id3Field {
                frame_id: "TXXX:bpm".into(),
                value: "100.0".into(),
            },
        ],
    };
    let track_context = TrackContext {
        track: Track::default(),
        feed: None,
    };

    let rows = aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());
    let tempo = rows
        .iter()
        .find_map(|row| match row {
            MetadataGridRow::Data(row) if row.field == "Tempo" => Some(row),
            MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
        })
        .expect("tempo row");

    assert_eq!(
        tempo.id3_value.as_deref(),
        Some("TBPM: 100.0\nTXXX:IBPM: 100.0\nTXXX:tempo: 100.0\nTXXX:bpm: 100.0")
    );

    let mut grouped = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
    grouped.insert("timing-seeking-audio-analysis-playback-control", rows);
    let aligned = aligned_id3_frame_ids(&result, &grouped);
    let used =
        super::used_id3_fields_for_group(&result, "descriptive-technical-rights-text", &aligned);
    assert!(
        used.is_empty(),
        "tempo aliases should not be repeated as separate raw ID3 rows"
    );
}

#[test]
fn sort_order_aliases_are_grouped_with_primary_rows() {
    let result = TagCompareResult {
        path: String::new(),
        rows: vec![
            ComparisonRow {
                field: "Title",
                source_value: Some("The Platform".into()),
                tag_value: Some("The Platform".into()),
                status: ComparisonStatus::Match,
            },
            ComparisonRow {
                field: "Artist",
                source_value: Some("HeyCitizen".into()),
                tag_value: Some("HeyCitizen".into()),
                status: ComparisonStatus::Match,
            },
            ComparisonRow {
                field: "Album/Feed",
                source_value: Some("Lofi Experience".into()),
                tag_value: Some("Lofi Experience".into()),
                status: ComparisonStatus::Match,
            },
        ],
        file_image: None,
        contributors: Vec::new(),
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![
            Id3Field {
                frame_id: "TIT2".into(),
                value: "The Platform".into(),
            },
            Id3Field {
                frame_id: "TSOT".into(),
                value: "Platform, The".into(),
            },
            Id3Field {
                frame_id: "TPE1".into(),
                value: "HeyCitizen".into(),
            },
            Id3Field {
                frame_id: "TSOP".into(),
                value: "Citizen, Hey".into(),
            },
            Id3Field {
                frame_id: "TALB".into(),
                value: "Lofi Experience".into(),
            },
            Id3Field {
                frame_id: "TSOA".into(),
                value: "Experience, Lofi".into(),
            },
        ],
    };
    let track_context = TrackContext {
        track: Track::default(),
        feed: None,
    };

    let rows = aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());
    let title = rows
        .iter()
        .find_map(|row| match row {
            MetadataGridRow::Data(row) if row.field == "Title" => Some(row),
            MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
        })
        .expect("title row");
    let artist = rows
        .iter()
        .find_map(|row| match row {
            MetadataGridRow::Data(row) if row.field == "Artist" => Some(row),
            MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
        })
        .expect("artist row");
    let album = rows
        .iter()
        .find_map(|row| match row {
            MetadataGridRow::Data(row) if row.field == "Album/Feed" => Some(row),
            MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
        })
        .expect("album row");

    assert_eq!(
        title.id3_value.as_deref(),
        Some("TIT2: The Platform\nTSOT: Platform, The")
    );
    assert_eq!(
        artist.id3_value.as_deref(),
        Some("TPE1: HeyCitizen\nTSOP: Citizen, Hey")
    );
    assert_eq!(
        album.id3_value.as_deref(),
        Some("TALB: Lofi Experience\nTSOA: Experience, Lofi")
    );
    assert_eq!(title.id3_status, ComparisonStatus::Match);

    let mut grouped = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
    grouped.insert("identification-release-structure", rows);
    let aligned = aligned_id3_frame_ids(&result, &grouped);
    let used =
        super::used_id3_fields_for_group(&result, "descriptive-technical-rights-text", &aligned);
    assert!(
        used.is_empty(),
        "sort-order aliases should not also appear as separate raw ID3 rows"
    );
}

#[test]
fn contributor_related_id3_frames_roll_up_into_contributors_row() {
    let result = TagCompareResult {
        path: String::new(),
        rows: Vec::new(),
        file_image: None,
        contributors: vec![
            crate::api::Contributor {
                name: Some("Alice".into()),
                role: Some("guitarist".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("Bob".into()),
                role: Some("audio engineer".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("Cara".into()),
                role: Some("composer".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("Dana".into()),
                role: Some("musician".into()),
                ..Default::default()
            },
        ],
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![
            Id3Field {
                frame_id: "TXXX:MUSICIANCREDITS".into(),
                value: "guitar: Alice / musician: Dana".into(),
            },
            Id3Field {
                frame_id: "TCOM".into(),
                value: "Cara".into(),
            },
            Id3Field {
                frame_id: "TIPL".into(),
                value: "engineer: Bob".into(),
            },
            Id3Field {
                frame_id: "TMCL".into(),
                value: "guitar: Alice".into(),
            },
            Id3Field {
                frame_id: "TPE1".into(),
                value: "Dana".into(),
            },
        ],
    };
    let track_context = TrackContext {
        track: Track::default(),
        feed: None,
    };

    let rows = aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());
    let contributors = rows
        .iter()
        .find_map(|row| match row {
            MetadataGridRow::Data(row) if row.field == "Contributors" => Some(row),
            MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
        })
        .expect("contributors row");

    assert_eq!(
        display_metadata_value(
            "Contributors",
            contributors
                .id3_value
                .as_deref()
                .expect("contributors value")
        ),
        "Alice: guitar\nBob: engineer\nCara: composer\nDana: musician"
    );
    assert_eq!(contributors.id3_status, ComparisonStatus::Match);

    let mut grouped = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
    grouped.insert("people-credits", rows);
    let aligned = aligned_id3_frame_ids(&result, &grouped);
    let used = super::used_id3_fields_for_group(&result, "people-credits", &aligned);
    assert!(
        used.is_empty(),
        "contributor-related ID3 frames should stay grouped under Contributors"
    );
}

#[test]
fn rss_and_musicbrainz_rows_use_semantic_groups() {
    assert_eq!(metadata_field_group_key("Artist"), "people-credits");
    assert_eq!(metadata_field_group_key("Website"), "url-link-frames");
    assert_eq!(
        metadata_field_group_key("ISRC"),
        "identification-release-structure"
    );
    assert_eq!(
        metadata_field_group_key("Value Routes"),
        "music-disc-acquisition-commerce"
    );
}

#[test]
fn release_date_prefers_item_then_feed_then_oldest_item_pubdate() {
    let track_context = TrackContext {
        track: Track {
            pub_date: Some(1_704_067_200),
            ..Default::default()
        },
        feed: Some(Feed {
            release_date: Some(1_672_531_200),
            oldest_item_at: Some(1_640_995_200),
            ..Default::default()
        }),
    };
    assert_eq!(
        super::musicindex_release_date(&track_context).as_deref(),
        Some("Jan 1, 2024")
    );

    let track_context = TrackContext {
        track: Track {
            pub_date: Some(1_704_067_200),
            ..Default::default()
        },
        feed: Some(Feed {
            oldest_item_at: Some(1_640_995_200),
            ..Default::default()
        }),
    };
    assert_eq!(
        super::musicindex_release_date(&track_context).as_deref(),
        Some("Jan 1, 2024")
    );

    let track_context = TrackContext {
        track: Track::default(),
        feed: Some(Feed {
            release_date: Some(1_672_531_200),
            oldest_item_at: Some(1_640_995_200),
            ..Default::default()
        }),
    };
    assert_eq!(
        super::musicindex_release_date(&track_context).as_deref(),
        Some("Jan 1, 2023")
    );
}

#[test]
fn musicbrainz_rows_align_with_id3_and_rss_equivalents() {
    let track_context = TrackContext {
        track: Track {
            title: Some("Song".into()),
            track_artist: Some("Artist".into()),
            track_number: Some(4),
            duration_secs: Some(199),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("isrc".into()),
                value: Some("USRC17607839".into()),
                ..Default::default()
            }]),
            ..Default::default()
        },
        feed: None,
    };
    let result = TagCompareResult {
        path: String::new(),
        rows: vec![
            ComparisonRow {
                field: "Title",
                source_value: Some("Song".into()),
                tag_value: Some("Song".into()),
                status: ComparisonStatus::Match,
            },
            ComparisonRow {
                field: "Artist",
                source_value: Some("Artist".into()),
                tag_value: Some("Artist".into()),
                status: ComparisonStatus::Match,
            },
            ComparisonRow {
                field: "Track #",
                source_value: Some("4".into()),
                tag_value: Some("4".into()),
                status: ComparisonStatus::Match,
            },
        ],
        file_image: None,
        contributors: Vec::new(),
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![Id3Field {
            frame_id: "TSRC".into(),
            value: "USRC17607839".into(),
        }],
    };
    let candidate = MusicBrainzCandidate {
        recording_id: "recording-id".into(),
        track_length_ms: Some(199_000),
        isrcs: vec!["USRC17607839".into()],
        ..Default::default()
    };

    let rows = musicbrainz_remainder_rows(&candidate, &track_context, Some(&result));
    let isrc_row = rows
        .iter()
        .find(|row| row.field == "ISRC")
        .expect("ISRC row should be present");
    assert_eq!(isrc_row.rss_value.as_deref(), Some("USRC17607839"));
    assert_eq!(isrc_row.id3_frame.as_deref(), Some("TSRC"));
    assert_eq!(isrc_row.id3_value.as_deref(), Some("USRC17607839"));
}

#[test]
fn id3_frame_version_classifies_frame_generations() {
    assert_eq!(id3_frame_version("TT2"), Id3FrameVersion::V22);
    assert_eq!(id3_frame_version("TYER"), Id3FrameVersion::V23Only);
    assert_eq!(id3_frame_version("TDRC"), Id3FrameVersion::V24Only);
    assert_eq!(id3_frame_version("TIT2"), Id3FrameVersion::V23V24);
    assert_eq!(id3_frame_version("ZZZZ"), Id3FrameVersion::Unknown);
    assert_eq!(
        id3_frame_group_key("TYER"),
        "descriptive-technical-rights-text"
    );
}

#[test]
fn drag_value_does_not_require_source_frame_hint() {
    let row = AlignedCompareRow {
        row_id: TrackMetadataGridVm::compare_row_id("RSS feed guid"),
        field: "RSS feed guid".into(),
        rss_value: Some("feed-guid".into()),
        id3_value: None,
        id3_frame: None,
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingTag,
        musicbrainz_status: ComparisonStatus::MissingTag,
    };

    let drag = metadata_drag_value(&row, MetadataColumn::Rss)
        .expect("RSS values without source ID3 hints should still be draggable");
    assert_eq!(drag.value, "feed-guid");
    assert_eq!(
        drag.frame, "",
        "the ID3 target cell supplies the destination frame on drop"
    );
}

#[test]
fn drag_copy_formats_values_for_id3v24_target_frames() {
    assert_eq!(
        format_drag_value_for_id3v24("TRCK", "Track #", None, "3 / 12").as_deref(),
        Some("3/12")
    );
    assert_eq!(
        format_drag_value_for_id3v24(
            "TXXX:MusicIndex Contributors",
            "Contributors",
            None,
            " Alice \0 Bob ",
        )
        .as_deref(),
        Some("Alice   Bob")
    );
    assert_eq!(
        format_drag_value_for_id3v24("TIT2", "Title", None, " \0 "),
        None
    );
    assert_eq!(
        format_drag_value_for_id3v24("TIT2", "Title", None, " - Song").as_deref(),
        Some("Song")
    );
    assert_eq!(
        format_drag_value_for_id3v24("TRCK", "Track #", Some("3/12"), "4").as_deref(),
        Some("4/12")
    );
    assert_eq!(
        format_drag_value_for_id3v24("TRCK", "Total tracks", Some("4"), "12").as_deref(),
        Some("4/12")
    );
    assert_eq!(
        format_drag_value_for_id3v24("TRCK", "Total tracks", None, "12"),
        None
    );
    assert_eq!(
        format_drag_value_for_id3v24("TDRC", "Release date", None, "Dec 8, 2025").as_deref(),
        Some("2025-12-08")
    );
    assert_eq!(
        format_drag_value_for_id3v24("TYER", "Release year", None, "Dec 8, 2025").as_deref(),
        Some("2025")
    );
    assert_eq!(
        format_drag_value_for_id3v24("WOAR", "Website", None, "https://a.test · https://b.test")
            .as_deref(),
        Some("https://a.test")
    );
    assert_eq!(
        format_drag_value_for_id3v24(
            "WXXX:Official audio",
            "Website",
            None,
            "https://a.test · https://b.test",
        )
        .as_deref(),
        Some("https://a.test")
    );
}

#[test]
fn all_compare_id3_hints_are_writable_id3v24_targets() {
    let fields = [
        "Title",
        "Artist",
        "Album/Feed",
        "Track #",
        "Publisher",
        "Nostr handle",
        "RSS feed nostr handle",
        "Label",
        "Website",
        "Tempo",
        "Release date",
        "Release year",
        "Duration",
        "Artwork",
        "Description",
        "Transcript",
        "Transcript text",
        "Contributors",
        "Composer",
        "Lyricist",
        "Lead performer",
        "Album artist",
        "Conductor",
        "Remixer",
        "Original artist",
        "Original lyricist",
        "Involved musicians",
        "Value Routes",
        "MusicBrainz recording",
        "MusicBrainz release",
        "MusicBrainz release group",
        "Release country",
        "Release status",
        "Barcode",
        "Release type",
        "Release secondary types",
        "Media",
        "Disc #",
        "Disc subtitle",
        "Total tracks",
        "ISRC",
    ];

    for field in fields {
        let hint = super::id3_frame_hint(field).expect("field should have an ID3 hint");
        assert!(
            id3v24_edit_label_is_writable(hint),
            "{field} should map to a writable ID3v2.4 target, got {hint}"
        );
    }
}

#[test]
fn auto_populates_missing_id3_from_rss_then_musicbrainz_without_source_conflicts() {
    let rows = vec![
        metadata_data_row(test_compare_row(
            "Title",
            Some("RSS Song"),
            None,
            Some("TIT2"),
            None,
        )),
        metadata_data_row(test_compare_row(
            "Artist",
            Some("RSS Artist"),
            None,
            Some("TPE1"),
            Some("MusicBrainz Artist"),
        )),
        metadata_data_row(test_compare_row(
            "Label",
            None,
            None,
            Some("TPUB"),
            Some("MusicBrainz Label"),
        )),
        metadata_data_row(test_compare_row(
            "Album/Feed",
            Some("Existing Album"),
            Some("Existing Album"),
            Some("TALB"),
            None,
        )),
        metadata_data_row(test_compare_row(
            "Release date",
            Some("2025"),
            None,
            Some("TDRC"),
            Some("2025"),
        )),
    ];

    let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new(), None);
    assert_eq!(pending.len(), 3);
    assert_eq!(pending["title"].value, "RSS Song");
    assert_eq!(pending["title"].source, MetadataColumn::Rss);
    assert_eq!(pending["label"].value, "MusicBrainz Label");
    assert_eq!(pending["label"].source, MetadataColumn::MusicBrainz);
    assert_eq!(pending["release-date"].value, "2025");
    assert_eq!(pending["release-date"].source, MetadataColumn::Rss);
    assert!(
        !pending.contains_key("artist"),
        "conflicting RSS and MusicBrainz values should remain manual"
    );
    assert!(
        !pending.contains_key("album-feed"),
        "existing ID3 values should not be auto-staged"
    );
}

#[test]
fn auto_populates_composite_track_number_targets_once_for_apply() {
    let rows = vec![
        metadata_data_row(test_compare_row(
            "Track #",
            Some("4"),
            None,
            Some("TRCK"),
            None,
        )),
        metadata_data_row(test_compare_row(
            "Total tracks",
            None,
            None,
            Some("TRCK"),
            Some("10"),
        )),
    ];

    let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new(), None);
    assert_eq!(pending["track"].value, "4/10");
    assert_eq!(pending["total-tracks"].value, "4/10");
    assert!(
        pending_id3_conflict_descriptions(&pending).is_empty(),
        "same target and same staged value should not be a conflict"
    );

    let edits = pending_id3_edits_for_apply(&pending);
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].frame_label, "TRCK");
    assert_eq!(edits[0].value, "4/10");
}

#[test]
fn auto_populates_multiple_woar_rows_for_distinct_outer_urls() {
    let rows = vec![metadata_data_row(test_compare_row(
        "Website",
        Some("https://rss.example/artist"),
        None,
        Some("WOAR"),
        Some("https://mb.example/artist"),
    ))];

    let expanded = expand_woar_metadata_rows(rows);
    let pending =
        auto_populated_pending_id3_edits(&expanded, &BTreeMap::new(), &BTreeSet::new(), None);
    assert_eq!(pending.len(), 2);
    assert_eq!(
        pending["compare:website"].value,
        "download for free (url, forward): https://rss.example/artist"
    );
    assert_eq!(pending["compare:website"].source, MetadataColumn::Rss);
    assert_eq!(
        pending["compare:website-2"].value,
        "https://mb.example/artist"
    );
    assert_eq!(
        pending["compare:website-2"].source,
        MetadataColumn::MusicBrainz
    );
    assert!(
        pending_id3_conflict_descriptions(&pending).is_empty(),
        "distinct WOAR URLs should be staged as repeatable URL frames"
    );

    let edits = pending_id3_edits_for_apply(&pending);
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().all(|edit| edit.frame_label == "WOAR"));
}

#[test]
fn wrapped_woar_url_counts_as_existing_website() {
    let url = "https://lnbeats.com/album/a2d2e313-9cbd-5169-b89c-ab7b33ecc33";
    let rows = vec![metadata_data_row(test_compare_row(
        "Website",
        Some(url),
        Some(&format!("download for free (url, forward): {url}")),
        Some("WOAR"),
        None,
    ))];

    let expanded = expand_woar_metadata_rows(rows);
    let row = expanded
        .iter()
        .find_map(|row| match row {
            MetadataGridRow::Data(row) => Some(row),
            MetadataGridRow::Group(_) => None,
        })
        .expect("website row");
    assert_eq!(row.id3_status, ComparisonStatus::Match);

    let pending =
        auto_populated_pending_id3_edits(&expanded, &BTreeMap::new(), &BTreeSet::new(), None);
    assert!(
        pending.is_empty(),
        "matching wrapped WOAR should not stage a duplicate website"
    );
}

#[test]
fn id3_compare_normalizes_dates_people_and_wrapped_urls() {
    assert_eq!(
        compare_id3_field_values("Release date", Some("Nov 7, 2023"), Some("2023-11-07")),
        ComparisonStatus::Match
    );
    assert_eq!(
        compare_id3_field_values(
            "Website",
            Some("https://example.test/album"),
            Some("download for free (url, forward): https://example.test/album")
        ),
        ComparisonStatus::Match
    );
    assert_eq!(
        compare_id3_field_values("Album artist", Some("HeyCitizen"), Some("Hey Citizen")),
        ComparisonStatus::Match
    );
    assert_eq!(
        compare_id3_field_values(
            "Performer [vocals]",
            Some("HeyCitizen / DuhLaurien / MaryKateUltra"),
            Some("Hey Citizen / DuhLaurien / Mary KateUltra")
        ),
        ComparisonStatus::Match
    );
}

#[test]
fn tagger_stages_transcript_url_as_sylt_and_uslt() {
    let context = TrackContext {
        track: Track {
            title: Some("Song".into()),
            source_links: Some(vec![SourceEntityLink {
                link_type: Some("transcript".into()),
                url: Some("https://example.com/song.srt".into()),
                extraction_path: Some("podcast:transcript@url".into()),
                ..Default::default()
            }]),
            ..Default::default()
        },
        feed: None,
    };

    let edits = crate::metadata_service::id3_edits_for_track_context(&context);
    assert!(edits.iter().any(|edit| {
        edit.frame_label == "SYLT:MusicIndex Transcript"
            && edit.value == "https://example.com/song.srt"
    }));
    assert!(edits.iter().any(|edit| {
        edit.frame_label == "USLT:MusicIndex Transcript"
            && edit.value == "https://example.com/song.srt"
    }));
}

#[test]
fn tagger_stages_nostr_handles_as_txxx() {
    let context = TrackContext {
        track: Track {
            title: Some("Song".into()),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1track".into()),
                ..Default::default()
            }]),
            ..Default::default()
        },
        feed: None,
    };

    let edits = crate::metadata_service::id3_edits_for_track_context(&context);
    assert!(edits
        .iter()
        .any(|edit| { edit.frame_label == "TXXX:RSS Nostr Handle" && edit.value == "npub1track" }));
}

#[test]
fn tagger_stages_musicindex_guids_as_txxx() {
    let context = TrackContext {
        track: Track {
            title: Some("Song".into()),
            track_guid: Some("track-guid".into()),
            feed_guid: Some("feed-guid".into()),
            ..Default::default()
        },
        feed: None,
    };

    let edits = crate::metadata_service::id3_edits_for_track_context(&context);
    assert!(edits.iter().any(|edit| {
        edit.frame_label == "TXXX:MusicIndex Track Guid" && edit.value == "track-guid"
    }));
    assert!(edits.iter().any(|edit| {
        edit.frame_label == "TXXX:MusicIndex Feed Guid" && edit.value == "feed-guid"
    }));
}

#[test]
fn contributors_map_to_picard_like_people_frames() {
    let contributors = vec![
        crate::api::Contributor {
            name: Some("Alice".into()),
            role: Some("guitarist".into()),
            ..Default::default()
        },
        crate::api::Contributor {
            name: Some("Bob".into()),
            role: Some("audio engineer".into()),
            ..Default::default()
        },
        crate::api::Contributor {
            name: Some("Cara".into()),
            role: Some("composer".into()),
            ..Default::default()
        },
        crate::api::Contributor {
            name: Some("Dana".into()),
            role: Some("musician".into()),
            ..Default::default()
        },
        crate::api::Contributor {
            name: Some("Band".into()),
            role: Some("album artist".into()),
            ..Default::default()
        },
        crate::api::Contributor {
            name: Some("Eli".into()),
            role: Some("Performer [keyboards]".into()),
            ..Default::default()
        },
    ];

    let rows = contributor_id3_rows(&contributors);
    assert!(rows.iter().any(|(field, frame, value)| {
        field == "Performer [guitar]" && *frame == "TMCL" && value == "Alice"
    }));
    assert!(rows
        .iter()
        .any(|(_, frame, value)| { *frame == "TIPL" && value == "engineer: Bob" }));
    assert!(rows
        .iter()
        .any(|(_, frame, value)| { *frame == "TCOM" && value == "Cara" }));
    assert!(rows
        .iter()
        .any(|(_, frame, value)| { *frame == "TPE1" && value == "Dana" }));
    assert!(rows
        .iter()
        .any(|(_, frame, value)| { *frame == "TPE2" && value == "Band" }));
    assert!(rows.iter().any(|(field, frame, value)| {
        field == "Performer [keyboards]" && *frame == "TMCL" && value == "Eli"
    }));

    let musicindex = musicindex_contributors_id3_value(&contributors)
        .expect("contributors should have a MusicIndex ID3 payload");
    assert_eq!(
            musicindex,
            "guitarist: Alice / audio engineer: Bob / composer: Cara / musician: Dana / album artist: Band / Performer [keyboards]: Eli"
        );
    assert_eq!(
            display_metadata_value("Contributors", &musicindex),
            "Alice: guitarist\nBand: album artist\nBob: audio engineer\nCara: composer\nDana: musician\nEli: Performer [keyboards]"
        );
}

#[test]
fn value_routes_keep_json_payload_but_display_pretty() {
    let value = r#"[{"recipient_name":"Alice","route_type":"node","split":75.0,"fee":false,"address":"abc","custom_key":null,"custom_value":null},{"recipient_name":"Hosting","route_type":"node","split":25.0,"fee":true,"address":"def","custom_key":null,"custom_value":null}]"#;
    assert_eq!(
            display_metadata_value("Value Routes", value),
            "[\n  {\n    \"recipient_name\": \"Alice\",\n    \"route_type\": \"node\",\n    \"split\": 75.0,\n    \"fee\": false,\n    \"address\": \"abc\",\n    \"custom_key\": null,\n    \"custom_value\": null\n  },\n  {\n    \"recipient_name\": \"Hosting\",\n    \"route_type\": \"node\",\n    \"split\": 25.0,\n    \"fee\": true,\n    \"address\": \"def\",\n    \"custom_key\": null,\n    \"custom_value\": null\n  }\n]"
        );
}

#[test]
fn tmcl_rows_match_picard_like_performer_fields() {
    let result = TagCompareResult {
        path: String::new(),
        rows: Vec::new(),
        file_image: None,
        contributors: vec![
            crate::api::Contributor {
                name: Some("HeyCitizen".into()),
                role: Some("vocal".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("DuhLaurien".into()),
                role: Some("vocals".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("MaryKateUltra".into()),
                role: Some("vocalist".into()),
                ..Default::default()
            },
        ],
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![Id3Field {
            frame_id: "TMCL".into(),
            value: "Hey Citizen - vocals / vocals:DuhLaurien / vocals: Mary KateUltra".into(),
        }],
    };
    let rows = aligned_compare_rows(
        &result,
        &TrackContext {
            track: Track::default(),
            feed: None,
        },
        None,
        false,
        &BTreeSet::new(),
    );
    let performer = rows
        .iter()
        .find_map(|row| match row {
            MetadataGridRow::Data(row) if row.field == "Performer [vocals]" => Some(row),
            MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
        })
        .expect("performer row");

    assert_eq!(performer.id3_status, ComparisonStatus::Match);
    assert_eq!(
        performer.id3_value.as_deref(),
        Some("Hey Citizen · DuhLaurien · Mary KateUltra")
    );
}

#[test]
fn transcript_rows_visible_even_when_content_group_collapsed() {
    let result = TagCompareResult {
        path: String::new(),
        rows: Vec::new(),
        file_image: None,
        contributors: Vec::new(),
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![Id3Field {
            frame_id: "USLT:MusicIndex Transcript".into(),
            value: "line one\nline two".into(),
        }],
    };
    let track_context = TrackContext {
        track: Track {
            source_links: Some(vec![SourceEntityLink {
                link_type: Some("transcript".into()),
                url: Some("https://example.test/transcript.srt".into()),
                extraction_path: Some("podcast:transcript@url".into()),
                ..Default::default()
            }]),
            ..Default::default()
        },
        feed: None,
    };

    // Transcript rows should be visible even when the content group is collapsed
    let collapsed = aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());
    let collapsed_fields = collapsed
        .iter()
        .filter_map(|row| match row {
            MetadataGridRow::Data(row) => Some(row.field.as_str()),
            MetadataGridRow::Group(_) => None,
        })
        .collect::<Vec<_>>();
    assert!(collapsed_fields.contains(&"Transcript"));
    assert!(collapsed_fields.contains(&"Transcript text"));

    let transcript_text = collapsed
        .iter()
        .find_map(|row| match row {
            MetadataGridRow::Data(row) if row.field == "Transcript text" => Some(row),
            MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
        })
        .expect("transcript text row");
    assert_eq!(transcript_text.id3_status, ComparisonStatus::Match);
}

#[test]
fn suppressed_auto_id3_rows_are_not_reselected() {
    let rows = vec![metadata_data_row(test_compare_row(
        "Title",
        Some("RSS Song"),
        None,
        Some("TIT2"),
        None,
    ))];
    let suppressed = BTreeSet::from(["title".to_string()]);

    let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &suppressed, None);
    assert!(pending.is_empty());
}

#[test]
fn track_rows_show_parent_feed_total_tracks_after_track_number() {
    let track_context = TrackContext {
        track: Track {
            track_number: Some(4),
            ..Default::default()
        },
        feed: Some(Feed {
            episode_count: Some(10),
            ..Default::default()
        }),
    };

    let rows = track_metadata_rows(&track_context, None, false);
    let fields = rows
        .iter()
        .filter_map(|row| match row {
            MetadataGridRow::Data(row) => Some(row.field.as_str()),
            MetadataGridRow::Group(_) => None,
        })
        .collect::<Vec<_>>();
    assert!(fields.contains(&"Track #"), "track row should exist");
    assert!(
        !fields.contains(&"Total tracks"),
        "total tracks should be merged into Track # row"
    );
}

#[test]
fn guid_rows_only_read_matching_txxx_frames() {
    let result = TagCompareResult {
        path: String::new(),
        rows: Vec::new(),
        file_image: None,
        contributors: Vec::new(),
        value_routes: Vec::new(),
        total_tracks: None,
        format: None,
        id3_fields: vec![
            Id3Field {
                frame_id: "TXXX:MusicIndex Track Guid".into(),
                value: "track-guid".into(),
            },
            Id3Field {
                frame_id: "TXXX:MusicIndex Feed Guid".into(),
                value: "feed-guid".into(),
            },
            Id3Field {
                frame_id: "TXXX:MusicIndex Value Routes".into(),
                value: "[4 items]".into(),
            },
            Id3Field {
                frame_id: "TXXX:MusicIndex Contributors".into(),
                value: "musician: HeyCitizen".into(),
            },
        ],
    };

    assert_eq!(
        super::id3_value_for_field("RSS track guid", &result).as_deref(),
        Some("track-guid")
    );
    assert_eq!(
        super::id3_value_for_field("RSS feed guid", &result).as_deref(),
        Some("feed-guid")
    );
}

#[test]
fn pending_id3_conflicts_detect_duplicate_effective_targets() {
    let edits = BTreeMap::from([
        (
            "track".into(),
            PendingId3Edit {
                field: "Track #".into(),
                frame: "TRCK".into(),
                value: "4".into(),
                source: MetadataColumn::Rss,
            },
        ),
        (
            "total".into(),
            PendingId3Edit {
                field: "Total tracks".into(),
                frame: "TRCK".into(),
                value: "10".into(),
                source: MetadataColumn::MusicBrainz,
            },
        ),
        (
            "release".into(),
            PendingId3Edit {
                field: "MusicBrainz release".into(),
                frame: "TXXX:MusicBrainz Album Id".into(),
                value: "release-id".into(),
                source: MetadataColumn::MusicBrainz,
            },
        ),
    ]);

    assert_eq!(
        pending_id3_target_key("TXXX:MusicBrainz Album Id"),
        "TXXX:musicbrainz album id"
    );
    assert_eq!(
        pending_id3_conflict_descriptions(&edits),
        vec!["TRCK (Total tracks, Track #)"]
    );
}

fn test_compare_row(
    field: &str,
    rss_value: Option<&str>,
    id3_value: Option<&str>,
    id3_frame: Option<&str>,
    musicbrainz_value: Option<&str>,
) -> AlignedCompareRow {
    AlignedCompareRow {
        row_id: TrackMetadataGridVm::compare_row_id(field),
        field: field.into(),
        rss_value: rss_value.map(str::to_string),
        id3_value: id3_value.map(str::to_string),
        id3_frame: id3_frame.map(str::to_string),
        musicbrainz_value: musicbrainz_value.map(str::to_string),
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingTag,
        musicbrainz_status: ComparisonStatus::MissingTag,
    }
}
