use super::*;
use crate::api::{Publisher, Recording, Release};
use crate::view_models::entity_detail::{EntityActionKind, EntityActionTarget, EntityActionVm};
use crate::view_models::ActionStatusMessageDisplay;
use crate::views::FeedRef;

fn assert_width_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON);
}

fn playlist(name: &str) -> db::Playlist {
    db::Playlist {
        id: 1,
        name: name.into(),
        description: None,
        track_count: 0,
        created_at: 0,
        updated_at: 0,
    }
}

#[test]
fn artist_display_uses_counts_area_and_image() {
    let detail = EntityDetail::Artist(Artist {
        name: Some("The Artist".into()),
        track_count: Some(1),
        feed_count: Some(2),
        area: Some("Canada".into()),
        begin_year: Some(1999),
        image_url: Some("https://example.test/a.png".into()),
        ..Artist::default()
    });

    assert_eq!(
        ResultRowVm::new("artist-id", Some(&detail)).display(),
        ResultRowDisplay {
            element_id: String::new(),
            kind_label: String::new(),
            line1: "The Artist".into(),
            line2: "1 track · 2 feeds".into(),
            line3: "Canada".into(),
            image_url: Some("https://example.test/a.png".into()),
        }
    );
}

#[test]
fn artist_display_falls_back_to_active_years_then_entity_id() {
    let detail = EntityDetail::Artist(Artist {
        begin_year: Some(2001),
        end_year: None,
        ..Artist::default()
    });

    let display = ResultRowVm::new("artist-id", Some(&detail)).display();
    assert_eq!(display.line1, "artist-id");
    assert_eq!(display.line3, "2001-");
}

#[test]
fn feed_display_uses_title_fallbacks_and_episode_count() {
    let detail = EntityDetail::Feed(Feed {
        name: Some("Feed Name".into()),
        feed_guid: Some("feed-guid".into()),
        release_artist: Some("Release Artist".into()),
        episode_count: Some(12),
        image_url: Some("https://example.test/f.png".into()),
        ..Feed::default()
    });

    assert_eq!(
        ResultRowVm::new("feed-id", Some(&detail)).display(),
        ResultRowDisplay {
            element_id: String::new(),
            kind_label: String::new(),
            line1: "Feed Name".into(),
            line2: "Release Artist".into(),
            line3: "12 tracks".into(),
            image_url: Some("https://example.test/f.png".into()),
        }
    );
}

#[test]
fn recent_feed_tile_vm_uses_current_recent_feed_response_labels() {
    let response: api::RecentFeedsResponse = serde_json::from_str(
        r#"{
                "data": [{
                    "feed_guid": "495c0d0b-f576-5d12-a76a-d806f2e19b7e",
                    "feed_url": "https://feeds.fountain.fm/ttc59BjLMAAPgxnP2fy2",
                    "title": "Is Anybody There?",
                    "raw_medium": "music",
                    "release_artist": "The Paisley Daze",
                    "release_artist_sort": null,
                    "release_date": 1777630024,
                    "release_kind": "unknown",
                    "description": null,
                    "image_url": "https://feeds.fountain.fm/cover.jpg",
                    "publisher_text": "The Paisley Daze",
                    "language": "en",
                    "explicit": false,
                    "episode_count": 1,
                    "newest_item_at": 1777630023,
                    "oldest_item_at": 1777630023,
                    "created_at": 1777650856,
                    "updated_at": 1777650856
                }],
                "pagination": {
                    "cursor": "next",
                    "has_more": true
                }
            }"#,
    )
    .expect("recent feeds response should deserialize");

    let feed = response.data.first().expect("fixture includes one feed");
    let vm = RecentFeedTileVm::new(feed);
    let display = vm.display();

    assert_eq!(display.id, "495c0d0b-f576-5d12-a76a-d806f2e19b7e");
    assert_eq!(
        display.feed_list_tile_id,
        "feed-tile:495c0d0b-f576-5d12-a76a-d806f2e19b7e"
    );
    assert_eq!(
        display.recent_tile_id,
        "recent-tile:495c0d0b-f576-5d12-a76a-d806f2e19b7e"
    );
    assert_eq!(
        display.podroll_tile_id,
        "podroll-tile:495c0d0b-f576-5d12-a76a-d806f2e19b7e"
    );
    assert_eq!(display.title, "Is Anybody There?");
    assert_eq!(display.subtitle.as_deref(), Some("The Paisley Daze"));
    assert_eq!(display.episode_note.as_deref(), Some("1 tracks"));
    assert_eq!(
        display.image_url.as_deref(),
        Some("https://feeds.fountain.fm/cover.jpg")
    );
}

#[test]
fn recent_feed_tile_vm_falls_back_to_publisher_for_subtitle() {
    let feed = Feed {
        title: Some("Feed Title".into()),
        publisher_text: Some("Publisher".into()),
        ..Feed::default()
    };
    let vm = RecentFeedTileVm::new(&feed);
    let display = vm.display();

    assert_eq!(display.title, "Feed Title");
    assert_eq!(display.subtitle.as_deref(), Some("Publisher"));
}

#[test]
fn recent_feed_tile_vm_projects_id_and_episode_note() {
    let feed = Feed {
        feed_guid: Some("feed-guid".into()),
        episode_count: Some(0),
        ..Feed::default()
    };
    let display = RecentFeedTileVm::new(&feed).display();

    assert_eq!(display.id, "feed-guid");
    assert_eq!(display.feed_list_tile_id, "feed-tile:feed-guid");
    assert_eq!(display.recent_tile_id, "recent-tile:feed-guid");
    assert_eq!(display.podroll_tile_id, "podroll-tile:feed-guid");
    assert_eq!(display.episode_note.as_deref(), Some("0 tracks"));

    let feed = Feed {
        feed_guid: None,
        episode_count: None,
        ..Feed::default()
    };
    let display = RecentFeedTileVm::new(&feed).display();

    assert_eq!(display.id, "");
    assert_eq!(display.feed_list_tile_id, "feed-tile:");
    assert_eq!(display.recent_tile_id, "recent-tile:");
    assert_eq!(display.podroll_tile_id, "podroll-tile:");
    assert_eq!(display.episode_note, None);
}

#[test]
fn podroll_section_display_projects_heading_and_scroll_id() {
    assert_eq!(
        SearchViewModel::podroll_section_display("feed-1"),
        PodrollSectionDisplay {
            heading_label: "Podroll",
            scroll_id: "podroll-scroll:feed-1".into(),
        }
    );
}

#[test]
fn recent_feed_tile_vm_does_not_emit_placeholder_ellipsis() {
    let feed = Feed {
        title: Some(" … ".into()),
        name: Some("...".into()),
        release_artist: Some("...".into()),
        publisher_text: Some("Publisher".into()),
        feed_guid: Some("feed-guid".into()),
        ..Feed::default()
    };
    let display = RecentFeedTileVm::new(&feed).display();

    assert_eq!(display.title, "feed-guid");
    assert_eq!(display.subtitle.as_deref(), Some("Publisher"));
    assert_ne!(display.title, "...");
    assert_ne!(display.subtitle.as_deref(), Some("..."));
}

#[test]
fn track_display_uses_track_vm_title_duration_and_artist_fallback() {
    let detail = EntityDetail::Track(Track {
        name: Some("Track Name".into()),
        duration_secs: Some(65),
        feed_title: Some("Feed Title".into()),
        release_artist: Some("Release Artist".into()),
        image_url: Some("https://example.test/t.png".into()),
        ..Track::default()
    });

    assert_eq!(
        ResultRowVm::new("track-id", Some(&detail)).display(),
        ResultRowDisplay {
            element_id: String::new(),
            kind_label: String::new(),
            line1: "Track Name – 1:05".into(),
            line2: "Unknown".into(),
            line3: "Feed Title by Release Artist".into(),
            image_url: Some("https://example.test/t.png".into()),
        }
    );
}

#[test]
fn publisher_display_keeps_no_image_contract() {
    let detail = EntityDetail::Publisher(Publisher {
        publisher_text: Some("Pub".into()),
        feed_count: Some(2),
        track_count: Some(3),
        ..Publisher::default()
    });

    assert_eq!(
        ResultRowVm::new("publisher-id", Some(&detail)).display(),
        ResultRowDisplay {
            element_id: String::new(),
            kind_label: String::new(),
            line1: "Pub".into(),
            line2: "2 feeds · 3 tracks".into(),
            line3: String::new(),
            image_url: None,
        }
    );
}

#[test]
fn fallback_rows_preserve_release_and_recording_images() {
    let release = EntityDetail::Release(Release {
        image_url: Some("https://example.test/release.png".into()),
        ..Release::default()
    });
    let recording = EntityDetail::Recording(Recording {
        image_url: Some("https://example.test/recording.png".into()),
        ..Recording::default()
    });

    assert_eq!(
        ResultRowVm::new("release-id", Some(&release))
            .display()
            .image_url
            .as_deref(),
        Some("https://example.test/release.png")
    );
    assert_eq!(
        ResultRowVm::new("recording-id", Some(&recording))
            .display()
            .image_url
            .as_deref(),
        Some("https://example.test/recording.png")
    );
    assert_eq!(ResultRowVm::new("bare-id", None).display().line1, "bare-id");
}

#[test]
fn visible_result_types_match_discover_scope() {
    assert!(search_result_type_is_visible("artist"));
    assert!(search_result_type_is_visible("feed"));
    assert!(search_result_type_is_visible("track"));
    assert!(!search_result_type_is_visible("publisher"));
}

#[test]
fn artist_rows_are_derived_from_feed_and_track_details() {
    let rows = vec![
        ResultRow::new(
            "track",
            "track-1",
            Some(EntityDetail::Track(Track {
                track_artist: Some("The Doerfels".into()),
                release_artist: Some("The Doerfels".into()),
                image_url: Some("https://example.test/track.png".into()),
                ..Track::default()
            })),
        ),
        ResultRow::new(
            "feed",
            "feed-1",
            Some(EntityDetail::Feed(Feed {
                release_artist: Some("The Doerfels".into()),
                image_url: Some("https://example.test/feed.png".into()),
                ..Feed::default()
            })),
        ),
        ResultRow::new(
            "artist",
            "other",
            Some(EntityDetail::Artist(Artist {
                name: Some("Other Artist".into()),
                ..Artist::default()
            })),
        ),
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
fn publisher_inspector_vm_falls_back_to_unknown_publisher_title() {
    let pub_ = Publisher::default();
    let vm = PublisherInspectorVm::new(&pub_);
    assert_eq!(vm.title(), "Unknown publisher");
}

#[test]
fn publisher_inspector_vm_uses_publisher_text_when_present() {
    let pub_ = Publisher {
        publisher_text: Some("Acme Audio".into()),
        ..Publisher::default()
    };
    let vm = PublisherInspectorVm::new(&pub_);
    assert_eq!(vm.title(), "Acme Audio");
}

#[test]
fn publisher_inspector_vm_prefers_explicit_counts_over_collection_length() {
    let pub_ = Publisher {
        feed_count: Some(7),
        track_count: Some(42),
        feeds: Some(vec![Feed::default()]),
        tracks: Some(vec![Track::default(), Track::default()]),
        ..Publisher::default()
    };
    let vm = PublisherInspectorVm::new(&pub_);
    assert_eq!(vm.feed_count(), 7);
    assert_eq!(vm.track_count(), 42);
}

#[test]
fn publisher_inspector_vm_falls_back_to_collection_length_when_count_absent() {
    let pub_ = Publisher {
        feed_count: None,
        track_count: None,
        feeds: Some(vec![Feed::default(), Feed::default()]),
        tracks: Some(vec![Track::default(), Track::default(), Track::default()]),
        ..Publisher::default()
    };
    let vm = PublisherInspectorVm::new(&pub_);
    assert_eq!(vm.feed_count(), 2);
    assert_eq!(vm.track_count(), 3);
}

#[test]
fn publisher_inspector_vm_falls_back_to_zero_when_neither_present() {
    let pub_ = Publisher::default();
    let vm = PublisherInspectorVm::new(&pub_);
    assert_eq!(vm.feed_count(), 0);
    assert_eq!(vm.track_count(), 0);
}

#[test]
fn publisher_inspector_vm_detail_rows_render_in_feeds_then_tracks_order() {
    let pub_ = Publisher {
        feed_count: Some(3),
        track_count: Some(5),
        ..Publisher::default()
    };
    let vm = PublisherInspectorVm::new(&pub_);
    let rows = vm.detail_rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ("Feeds".into(), "3".into()));
    assert_eq!(rows[1], ("Tracks".into(), "5".into()));
}

#[test]
fn publisher_inspector_vm_has_feed_list_only_when_feeds_present() {
    let pub_ = Publisher::default();
    let vm = PublisherInspectorVm::new(&pub_);
    assert!(!vm.has_feed_list());
    let pub_ = Publisher {
        feeds: Some(vec![Feed::default()]),
        ..Publisher::default()
    };
    let vm = PublisherInspectorVm::new(&pub_);
    assert!(vm.has_feed_list());
}

#[test]
fn action_row_vm_visibility_matches_feed_and_track_only() {
    assert!(ActionRowVm::new("feed", false, None, None).is_visible());
    assert!(ActionRowVm::new("track", false, None, None).is_visible());
    assert!(!ActionRowVm::new("artist", false, None, None).is_visible());
    assert!(!ActionRowVm::new("publisher", false, None, None).is_visible());
    assert!(!ActionRowVm::new("release", false, None, None).is_visible());
}

#[test]
fn action_row_vm_busy_label_distinguishes_remove_vs_download() {
    // local_subscription = Some(true) → "Removing..."
    let vm = ActionRowVm::new("feed", true, Some(true), None);
    assert_eq!(vm.subscription_button_label(), "Removing...");
    // local_subscription = Some(false) → "Downloading..."
    let vm = ActionRowVm::new("feed", true, Some(false), None);
    assert_eq!(vm.subscription_button_label(), "Downloading...");
    // local_subscription = None → "Downloading..." (matches the
    // `unwrap_or(false)` in the legacy renderer).
    let vm = ActionRowVm::new("feed", true, None, None);
    assert_eq!(vm.subscription_button_label(), "Downloading...");
}

#[test]
fn action_row_vm_idle_label_picks_noun_by_entity_type() {
    let vm = ActionRowVm::new("feed", false, Some(false), None);
    assert_eq!(vm.subscription_button_label(), "Download Feed");
    let vm = ActionRowVm::new("track", false, Some(false), None);
    assert_eq!(vm.subscription_button_label(), "Download Track");
    let vm = ActionRowVm::new("feed", false, Some(true), None);
    assert_eq!(vm.subscription_button_label(), "Remove Feed");
    let vm = ActionRowVm::new("track", false, Some(true), None);
    assert_eq!(vm.subscription_button_label(), "Remove Track");
}

#[test]
fn action_row_vm_idle_label_treats_unknown_local_subscription_as_downloadable() {
    let vm = ActionRowVm::new("feed", false, None, None);
    assert_eq!(vm.subscription_button_label(), "Download Feed");
}

#[test]
fn action_row_vm_add_to_playlist_label_uses_feed_noun() {
    let vm = ActionRowVm::new("feed", false, None, None);
    assert_eq!(vm.add_to_playlist_label(), "Add feed to playlist");
    let vm = ActionRowVm::new("track", false, None, None);
    assert_eq!(vm.add_to_playlist_label(), "Add to playlist");
}

#[test]
fn action_row_vm_inspector_playlist_display_projects_id_and_label() {
    let vm = ActionRowVm::new("feed", false, None, None);
    assert_eq!(
        vm.inspector_playlist_display("feed-1", "Add feed to playlist ▾"),
        SearchInspectorPlaylistDisplay {
            popover_id: "inspector-add:feed-1".into(),
            trigger_label: "Add feed to playlist ▾".into(),
        }
    );

    let vm = ActionRowVm::new("track", false, None, None);
    assert_eq!(
        vm.inspector_playlist_display("track-1", "Add to playlist"),
        SearchInspectorPlaylistDisplay {
            popover_id: "inspector-add:track-1".into(),
            trigger_label: "Add to playlist".into(),
        }
    );
    assert_eq!(
        vm.inspector_playlist_display("track-1", "").trigger_label,
        "Add to playlist"
    );
}

#[test]
fn action_row_vm_playlist_trigger_label_uses_release_action_when_available() {
    let vm = ActionRowVm::new("feed", false, None, None);
    let action = EntityActionVm::new(
        EntityActionKind::AddToPlaylist,
        EntityActionTarget::Feed(FeedRef::Musicindex("feed-1".into())),
        "Add release to playlist",
        crate::view_models::entity_detail::EntityActionTone::Secondary,
    );
    assert_eq!(
        vm.playlist_trigger_label(Some(&action)),
        "Add release to playlist"
    );
    assert_eq!(vm.playlist_trigger_label(None), "Add feed to playlist");

    let vm = ActionRowVm::new("track", false, None, None);
    assert_eq!(vm.playlist_trigger_label(Some(&action)), "Add to playlist");
}

#[test]
fn action_row_vm_projects_subscription_message_display() {
    let vm = ActionRowVm::new("feed", false, None, Some("Subscribed!"));
    assert_eq!(
        vm.subscription_message_display(),
        Some(ActionStatusMessageDisplay::neutral("Subscribed!"))
    );
    let vm = ActionRowVm::new("feed", false, None, Some("error: bad request"));
    assert_eq!(
        vm.subscription_message_display(),
        Some(ActionStatusMessageDisplay::danger(
            "error: bad request",
            crate::view_models::ActionStatusMessageWidth::Status,
        ))
    );
    let vm = ActionRowVm::new("feed", false, None, Some("Error: bad request"));
    assert_eq!(
        vm.subscription_message_display(),
        Some(ActionStatusMessageDisplay::danger(
            "Error: bad request",
            crate::view_models::ActionStatusMessageWidth::Status,
        ))
    );
    let vm = ActionRowVm::new("feed", false, None, None);
    assert_eq!(vm.subscription_message_display(), None);
}

#[test]
fn track_row_action_vm_key_prefers_enclosure_then_guid() {
    let track = Track {
        enclosure_url: Some("https://example.test/a.mp3".into()),
        track_guid: Some("guid".into()),
        ..Track::default()
    };
    let vm = TrackRowActionVm::new(&track, false, false);
    assert_eq!(vm.key(), "https://example.test/a.mp3");

    let track = Track {
        enclosure_url: None,
        track_guid: Some("guid".into()),
        ..Track::default()
    };
    let vm = TrackRowActionVm::new(&track, false, false);
    assert_eq!(vm.key(), "guid");
}

#[test]
fn track_row_action_vm_labels_match_download_state() {
    let track = Track::default();
    let vm = TrackRowActionVm::new(&track, false, true);
    assert_eq!(vm.busy_tooltip(), "Downloading...");
    assert_eq!(vm.primary_action().label, "Downloading...");
    assert!(vm.is_in_flight());

    let vm = TrackRowActionVm::new(&track, true, true);
    assert_eq!(vm.busy_tooltip(), "Removing...");
    assert_eq!(vm.primary_action().label, "Removing...");
}

#[test]
fn track_row_action_vm_download_display_projects_ids_and_tooltip() {
    let track = Track {
        enclosure_url: Some("https://example.test/a.mp3".into()),
        track_guid: Some("guid".into()),
        ..Track::default()
    };
    let vm = TrackRowActionVm::new(&track, false, true);
    assert_eq!(
        vm.download_display(),
        TrackRowDownloadDisplay {
            busy_indicator_id: "track-row-download-spin:https://example.test/a.mp3".into(),
            button_id: "track-row-download:https://example.test/a.mp3".into(),
            busy_tooltip: "Downloading...",
        }
    );

    let vm = TrackRowActionVm::new(&track, true, true);
    assert_eq!(vm.download_display().busy_tooltip, "Removing...");
}

#[test]
fn track_row_action_vm_projects_shared_action_state() {
    let track = Track {
        track_guid: Some("track-guid".into()),
        enclosure_url: Some("https://example.test/track.mp3".into()),
        ..Track::default()
    };
    let remote = TrackRowActionVm::new(&track, false, false).primary_action();
    assert_eq!(remote.kind, EntityActionKind::Download);
    assert_eq!(remote.label, "Download");
    assert_eq!(
        remote.tone,
        crate::view_models::entity_detail::EntityActionTone::Secondary
    );
    assert!(remote.enabled);

    let removing = TrackRowActionVm::new(&track, true, true).primary_action();
    assert_eq!(removing.kind, EntityActionKind::Remove);
    assert_eq!(removing.label, "Removing...");
    assert_eq!(
        removing.tone,
        crate::view_models::entity_detail::EntityActionTone::DestructiveQuiet
    );
    assert!(!removing.enabled);
}

#[test]
fn track_row_action_vm_disables_download_when_track_has_no_enclosure() {
    let track = Track {
        track_guid: Some("track-guid".into()),
        enclosure_url: None,
        ..Track::default()
    };
    let action = TrackRowActionVm::new(&track, false, false).primary_action();

    assert_eq!(action.kind, EntityActionKind::Download);
    assert!(!action.enabled);
}

#[test]
fn artist_rows_merge_case_insensitive_counts() {
    let rows = vec![
        ResultRow::new(
            "feed",
            "feed-1",
            Some(EntityDetail::Feed(Feed {
                release_artist: Some("Artist".into()),
                ..Feed::default()
            })),
        ),
        ResultRow::new(
            "track",
            "track-1",
            Some(EntityDetail::Track(Track {
                track_artist: Some("artist".into()),
                ..Track::default()
            })),
        ),
    ];

    let artist_rows = artist_rows_from_result_rows(&rows, None);

    assert_eq!(artist_rows.len(), 1);
    let Some(EntityDetail::Artist(artist)) = &artist_rows[0].detail else {
        panic!("expected artist detail");
    };
    assert_eq!(artist.name.as_deref(), Some("Artist"));
    assert_eq!(artist.feed_count, Some(1));
    assert_eq!(artist.track_count, Some(1));
}

#[test]
fn search_view_model_starts_with_all_filter_fuzzy_on_and_no_selection() {
    let vm = SearchViewModel::new();
    assert_eq!(vm.type_filter, 0);
    // Production default — `SearchApp::new` set fuzzy_search = true
    // and the VM mirrors that.
    assert!(vm.fuzzy_search);
    assert_eq!(vm.selected_key, None);
    assert_eq!(vm.inspector_origin, None);
}

#[test]
fn search_view_model_starts_with_idle_panes_and_no_in_flight_tracks() {
    let vm = SearchViewModel::new();
    assert!(!vm.loading);
    assert!(vm.status.is_empty());
    assert_eq!(vm.cursor, None);
    assert!(!vm.has_more);
    assert!(vm.in_flight_tracks.is_empty());
    assert!(!vm.recent_loading);
    assert!(vm.recent_status.is_empty());
    assert!(!vm.recent_loaded_once);
    assert_eq!(vm.recent_cursor, None);
    assert!(!vm.recent_has_more);
    assert!(!vm.is_resizing());
    assert_width_eq(vm.split_pane_width(), DEFAULT_SPLIT_PANE_WIDTH);
}

#[test]
fn track_inspector_header_vm_feed_link_url_falls_back_to_feed_guid() {
    let track = Track {
        feed_url: Some("https://example/x.rss".into()),
        feed_guid: Some("guid-1".into()),
        ..Track::default()
    };
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(vm.feed_link_url().as_deref(), Some("https://example/x.rss"));

    let track = Track {
        feed_url: None,
        feed_guid: Some("guid-1".into()),
        ..Track::default()
    };
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(vm.feed_link_url().as_deref(), Some("guid-1"));

    let track = Track::default();
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(vm.feed_link_url(), None);
}

#[test]
fn track_inspector_header_vm_feed_link_label_uses_feed_title_then_falls_back_to_guid() {
    let track = Track {
        feed_title: Some("Friendly Title".into()),
        feed_guid: Some("guid-1".into()),
        ..Track::default()
    };
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(vm.feed_link_label("guid-1"), "Friendly Title");

    // Empty / whitespace-only feed_title falls back to the guid arg.
    let track = Track {
        feed_title: Some("   ".into()),
        feed_guid: Some("guid-1".into()),
        ..Track::default()
    };
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(vm.feed_link_label("guid-1"), "guid-1");

    let track = Track {
        feed_title: None,
        ..Track::default()
    };
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(vm.feed_link_label("fallback"), "fallback");
}

#[test]
fn track_inspector_header_vm_projects_feed_link_display_contract() {
    let track = Track {
        feed_title: Some("Friendly Title".into()),
        feed_url: Some("https://example/x.rss".into()),
        feed_guid: Some("guid-1".into()),
        ..Track::default()
    };
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(
        vm.feed_link_display(),
        Some(TrackFeedLinkDisplay {
            element_id: "track-feed-link:guid-1".into(),
            guid: "guid-1".into(),
            label: "Friendly Title".into(),
            url: Some("https://example/x.rss".into()),
            tooltip: "guid-1".into(),
        })
    );

    let track = Track {
        feed_title: Some("   ".into()),
        feed_url: None,
        feed_guid: Some("guid-1".into()),
        ..Track::default()
    };
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(
        vm.feed_link_display(),
        Some(TrackFeedLinkDisplay {
            element_id: "track-feed-link:guid-1".into(),
            guid: "guid-1".into(),
            label: "guid-1".into(),
            url: Some("guid-1".into()),
            tooltip: "guid-1".into(),
        })
    );

    let track = Track {
        feed_guid: None,
        ..Track::default()
    };
    let vm = TrackInspectorHeaderVm::new(&track);
    assert_eq!(vm.feed_link_display(), None);
}

#[test]
fn lazy_panel_collapsible_toggle_starts_fetch_then_expands_on_loading() {
    let mut panel: LazyPanel<Vec<i32>> = LazyPanel::Hidden;
    let mut collapsed = true;

    let action = panel.begin_collapsible_toggle(&mut collapsed, false);
    assert_eq!(action, LazyPanelToggle::Fetch);
    assert!(action.should_fetch());
    assert!(action.should_notify());
    assert_eq!(panel, LazyPanel::Loading);
    assert!(!collapsed);

    // Re-collapse, then click while a fetch (or background prefetch)
    // is in flight: the disclosure expands again to reveal the loading
    // state, but we do not start a second fetch.
    collapsed = true;
    let action = panel.begin_collapsible_toggle(&mut collapsed, false);
    assert_eq!(action, LazyPanelToggle::Toggled);
    assert!(!action.should_fetch());
    assert!(action.should_notify());
    assert!(!collapsed);
    assert_eq!(panel, LazyPanel::Loading);

    panel = LazyPanel::Loaded(vec![1]);
    let action = panel.begin_collapsible_toggle(&mut collapsed, false);
    assert_eq!(action, LazyPanelToggle::Toggled);
    assert!(!action.should_fetch());
    assert!(action.should_notify());
    assert!(collapsed);

    panel = LazyPanel::Empty("No items".into());
    let action = panel.begin_collapsible_toggle(&mut collapsed, false);
    assert_eq!(action, LazyPanelToggle::Toggled);
    assert!(!collapsed);
}

#[test]
fn lazy_panel_force_toggle_only_never_starts_fetch() {
    let mut panel: LazyPanel<Vec<i32>> = LazyPanel::Hidden;
    let mut collapsed = true;

    let action = panel.begin_collapsible_toggle(&mut collapsed, true);

    assert_eq!(action, LazyPanelToggle::Toggled);
    assert_eq!(panel, LazyPanel::Hidden);
    assert!(!collapsed);
}

#[test]
fn lazy_panel_from_items_result_maps_empty_loaded_and_error() {
    assert_eq!(
        LazyPanel::from_items_result(Result::<Vec<i32>, &str>::Ok(Vec::new()), "No rows"),
        LazyPanel::Empty("No rows".into())
    );
    assert_eq!(
        LazyPanel::from_items_result(Result::<Vec<i32>, &str>::Ok(vec![1, 2]), "No rows"),
        LazyPanel::Loaded(vec![1, 2])
    );
    assert_eq!(
        LazyPanel::from_items_result(Result::<Vec<i32>, &str>::Err("offline"), "No rows"),
        LazyPanel::Empty("Error: offline".into())
    );
}

#[test]
fn lazy_panel_error_owns_error_prefix_display() {
    assert_eq!(
        LazyPanel::<Vec<i32>>::error("offline"),
        LazyPanel::Empty("Error: offline".into())
    );
}

#[test]
fn payment_route_vm_falls_back_to_unnamed_recipient() {
    let r = api::PaymentRoute::default();
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.recipient_name(), "Unnamed recipient");
}

#[test]
fn payment_route_vm_route_type_defaults_to_route() {
    let r = api::PaymentRoute::default();
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.route_type(), "route");
    let r = api::PaymentRoute {
        route_type: Some("lightning".into()),
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.route_type(), "lightning");
}

#[test]
fn payment_route_vm_projects_primary_summary() {
    let r = api::PaymentRoute::default();
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.summary(), "Unnamed recipient (route · 0% · split)");

    let r = api::PaymentRoute {
        recipient_name: Some("Alice".into()),
        route_type: Some("node".into()),
        split: Some(75.0),
        fee: Some(true),
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.summary(), "Alice (node · 75% · fee)");
}

#[test]
fn payment_route_vm_projects_address_without_coercing_presence() {
    let r = api::PaymentRoute {
        address: Some("lnbc1abc".into()),
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.address().as_deref(), Some("lnbc1abc"));

    let r = api::PaymentRoute {
        address: Some(String::new()),
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.address().as_deref(), Some(""));

    let r = api::PaymentRoute {
        address: None,
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.address(), None);
}

#[test]
fn payment_route_vm_projects_custom_fields_without_coercing_presence() {
    let r = api::PaymentRoute {
        custom_key: Some("pubkey".into()),
        custom_value: Some("abc".into()),
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(
        vm.custom_fields().as_deref(),
        Some("key pubkey · value abc")
    );

    let r = api::PaymentRoute {
        custom_key: Some(String::new()),
        custom_value: None,
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.custom_fields().as_deref(), Some("key "));

    let r = api::PaymentRoute {
        custom_key: None,
        custom_value: None,
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert_eq!(vm.custom_fields(), None);
}

#[test]
fn payment_route_vm_classifies_fee_vs_split() {
    let r = api::PaymentRoute::default();
    let vm = PaymentRouteVm::new(&r);
    assert!(!vm.is_fee());
    assert_eq!(vm.kind_label(), "split");
    assert_eq!(vm.group(), "Recipients");
    assert_eq!(
        PaymentRouteVm::group_display(vm.group()),
        PaymentRouteGroupDisplay {
            heading: "Recipients"
        }
    );

    let r = api::PaymentRoute {
        fee: Some(true),
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert!(vm.is_fee());
    assert_eq!(vm.kind_label(), "fee");
    assert_eq!(vm.group(), "Fees");
    assert_eq!(
        PaymentRouteVm::group_display(vm.group()),
        PaymentRouteGroupDisplay { heading: "Fees" }
    );
}

#[test]
fn payment_route_vm_split_value_defaults_to_zero() {
    let r = api::PaymentRoute::default();
    let vm = PaymentRouteVm::new(&r);
    assert!((vm.split() - 0.0).abs() < f64::EPSILON);
    let r = api::PaymentRoute {
        split: Some(50.0),
        ..api::PaymentRoute::default()
    };
    let vm = PaymentRouteVm::new(&r);
    assert!((vm.split() - 50.0).abs() < f64::EPSILON);
}

#[test]
fn search_view_model_starts_with_empty_snapshots() {
    let vm = SearchViewModel::new();
    assert!(vm.results.is_empty());
    assert!(vm.recent_feeds.is_empty());
    assert!(vm.playlists.is_empty());
}

#[test]
fn search_view_model_set_type_filter_updates_index_and_clears_when_unknown_type() {
    let mut vm = SearchViewModel::new();
    vm.set_type_filter(2);
    assert_eq!(vm.type_filter, 2);
    assert!(!vm.set_type_filter_if_changed(2));
    assert!(vm.set_type_filter_if_changed(3));
    assert_eq!(vm.type_filter, 3);
    // Out-of-range index stays at the prior value (caller is the
    // segmented control which knows its range).
    vm.set_type_filter(99);
    assert_eq!(vm.type_filter, 3);
    assert!(!vm.set_type_filter_if_changed(99));
    assert_eq!(vm.type_filter, 3);
}

#[test]
fn search_type_filter_options_project_labels_and_values() {
    assert_eq!(
        SearchViewModel::type_filter_options(),
        [
            SearchTypeFilterOptionDisplay {
                index: 0,
                button_id: "type-filter-all",
                label: "All",
                a11y_label: "Show all search result types",
                value: None,
            },
            SearchTypeFilterOptionDisplay {
                index: 1,
                button_id: "type-filter-artist",
                label: "Artist",
                a11y_label: "Show artist search results",
                value: Some("artist"),
            },
            SearchTypeFilterOptionDisplay {
                index: 2,
                button_id: "type-filter-feed",
                label: "Feed",
                a11y_label: "Show feed search results",
                value: Some("feed"),
            },
            SearchTypeFilterOptionDisplay {
                index: 3,
                button_id: "type-filter-track",
                label: "Track",
                a11y_label: "Show track search results",
                value: Some("track"),
            },
        ]
    );
    assert_eq!(SearchViewModel::type_filter_value(0), None);
    assert_eq!(SearchViewModel::type_filter_value(2), Some("feed"));
    assert_eq!(SearchViewModel::type_filter_value(99), None);
}

#[test]
fn search_view_model_toggle_fuzzy_search_round_trip() {
    let mut vm = SearchViewModel::new();
    // Starts true (production default). Toggling once turns it off.
    vm.toggle_fuzzy_search();
    assert!(!vm.fuzzy_search);
    vm.toggle_fuzzy_search();
    assert!(vm.fuzzy_search);
}

#[test]
fn search_view_model_inspector_origin_remembers_search_vs_recents() {
    let mut vm = SearchViewModel::new();
    vm.mark_inspector_from_search();
    assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Search));
    vm.mark_inspector_from_recents();
    assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Recents));
    vm.clear_inspector_origin();
    assert_eq!(vm.inspector_origin, None);
}

#[test]
fn search_view_model_select_and_clear_selection() {
    let mut vm = SearchViewModel::new();
    vm.select("track:abc");
    assert_eq!(vm.selected_key.as_deref(), Some("track:abc"));
    vm.clear_selection();
    assert_eq!(vm.selected_key, None);
}

#[test]
fn search_status_snapshot_prefixes_error_display() {
    let snapshot = SearchStatusSnapshot::from_text("Error: offline");
    assert_eq!(snapshot.text, "Error: offline");
    assert_eq!(snapshot.display_text, "\u{2717} Error: offline");
    assert!(snapshot.is_error);

    let snapshot = SearchStatusSnapshot::from_text("Ready");
    assert_eq!(snapshot.text, "Ready");
    assert_eq!(snapshot.display_text, "Ready");
    assert!(!snapshot.is_error);
}

#[test]
fn search_render_snapshot_projects_result_pane_display_labels() {
    let mut vm = SearchViewModel::new();
    vm.status = "Error: offline".into();
    vm.loading = true;
    vm.has_more = true;
    vm.type_filter = 2;
    vm.fuzzy_search = false;
    vm.results.push(ResultRow::new("feed", "feed-1", None));
    vm.select_result("feed", "feed-1");

    let snapshot = vm.render_snapshot(true, true);

    assert_eq!(snapshot.status.text, "Error: offline");
    assert_eq!(snapshot.status.display_text, "\u{2717} Error: offline");
    assert!(snapshot.status.is_error);
    assert!(!snapshot.status.is_empty());
    assert_eq!(snapshot.pane_display.heading, "Results");
    assert_eq!(snapshot.pane_display.search_button_label, "Search Index");
    assert_eq!(snapshot.pane_display.refresh_button_label, "Refresh");
    assert_eq!(snapshot.pane_display.fuzzy_toggle_label, "Fuzzy: Off");
    assert_eq!(snapshot.pane_display.empty_icon, "\u{1F50D}");
    assert_eq!(snapshot.pane_display.empty_label, "No results");
    assert_eq!(snapshot.pane_display.load_more_label, "Load more");
    assert_eq!(snapshot.sections.len(), 1);
    assert_eq!(snapshot.sections[0].display.heading, "Index");
    assert_eq!(snapshot.sections[0].rows.len(), 1);
    assert_eq!(snapshot.selected_key.as_deref(), Some("index:feed:feed-1"));
    assert_eq!(snapshot.type_filter, 2);
    assert_eq!(snapshot.index_controls, IndexControlsVisibility::Visible);
    assert!(!snapshot.show_recents_root);
    assert!(snapshot.show_recents_command);
    assert!(snapshot.loading);
    assert!(!snapshot.empty);
    assert!(snapshot.has_more);
    assert!(!snapshot.fuzzy_search);

    let empty_snapshot = SearchViewModel::new().render_snapshot(true, true);
    assert!(empty_snapshot.show_recents_root);
    assert!(!empty_snapshot.show_recents_command);
    assert!(empty_snapshot.empty);
    assert_eq!(
        empty_snapshot.index_controls,
        IndexControlsVisibility::Visible
    );
    assert!(empty_snapshot.status.is_empty());
    assert_eq!(empty_snapshot.status.display_text, "");
    assert_eq!(empty_snapshot.pane_display.split_pane_id, "pane-container");
    assert_eq!(
        empty_snapshot.pane_display.resize_handle_id,
        "resize-handle"
    );
    assert_eq!(empty_snapshot.pane_display.search_button_id, "search-btn");
    assert_eq!(empty_snapshot.pane_display.fuzzy_toggle_id, "fuzzy-toggle");
    assert_eq!(
        empty_snapshot.pane_display.results_scroll_id,
        "results-scroll"
    );
    assert_eq!(empty_snapshot.pane_display.load_more_button_id, "load-more");
    assert_eq!(empty_snapshot.pane_display.fuzzy_toggle_label, "Fuzzy: On");
}

#[test]
fn search_render_snapshot_groups_library_before_index_for_all_scope() {
    let mut vm = SearchViewModel::new();
    vm.active_filter = ContentFilter::All;
    vm.library_results = vec![ResultRow::local_library_track(
        42,
        EntityDetail::Track(Track {
            title: Some("Local Track".into()),
            ..Track::default()
        }),
    )];
    vm.results = vec![ResultRow::new("feed", "index-feed", None)];

    let snapshot = vm.render_snapshot(true, false);

    assert_eq!(snapshot.sections.len(), 2);
    assert_eq!(snapshot.sections[0].display.heading, "Library");
    assert_eq!(snapshot.sections[0].rows[0].key(), "library:track:42");
    assert_eq!(snapshot.sections[1].display.heading, "Index");
    assert_eq!(snapshot.sections[1].rows[0].key(), "index:feed:index-feed");
    assert!(!snapshot.empty);
}

#[test]
fn search_render_snapshot_applies_type_filter_to_library_and_index_sections() {
    let mut vm = SearchViewModel::new();
    vm.active_filter = ContentFilter::All;
    vm.set_type_filter(2);
    vm.library_results = vec![ResultRow::local_library_track(
        42,
        EntityDetail::Track(Track {
            title: Some("Local Track".into()),
            ..Track::default()
        }),
    )];
    vm.results = vec![
        ResultRow::new("track", "index-track", None),
        ResultRow::new("feed", "index-feed", None),
    ];

    let snapshot = vm.render_snapshot(true, false);

    assert_eq!(snapshot.sections.len(), 2);
    assert!(
        snapshot.sections[0].rows.is_empty(),
        "Feed filter should hide local track rows"
    );
    assert_eq!(snapshot.sections[1].rows.len(), 1);
    assert_eq!(snapshot.sections[1].rows[0].key(), "index:feed:index-feed");
}

#[test]
fn search_library_membership_display_projects_labels() {
    assert_eq!(
        SearchLibraryMembership::InLibrary.display(),
        SearchLibraryMembershipDisplay {
            label: "In Library",
            a11y_label: "Item is in the local library",
            is_in_library: true,
        }
    );
    assert_eq!(
        SearchLibraryMembership::NotInLibrary.display(),
        SearchLibraryMembershipDisplay {
            label: "Not in Library",
            a11y_label: "Item is not in the local library",
            is_in_library: false,
        }
    );
}

#[test]
fn search_render_snapshot_hides_index_controls_for_library_scope() {
    let mut vm = SearchViewModel::new();
    vm.active_filter = ContentFilter::Library;

    let snapshot = vm.render_snapshot(true, false);

    assert_eq!(snapshot.index_controls, IndexControlsVisibility::Hidden);
    assert_eq!(snapshot.sections.len(), 1);
    assert_eq!(snapshot.sections[0].display.heading, "Library");
}

#[test]
fn search_render_snapshot_keeps_recent_feeds_reachable_after_search() {
    let mut vm = SearchViewModel::new();
    vm.results.push(ResultRow::new("feed", "feed-1", None));
    let snapshot = vm.render_snapshot(true, false);

    assert!(!snapshot.show_recents_root);
    assert!(snapshot.show_recents_command);
}

#[test]
fn recent_feeds_snapshot_projects_panel_display_labels() {
    let mut vm = SearchViewModel::new();
    vm.recent_feeds.push(Feed {
        feed_guid: Some("feed-1".into()),
        ..Feed::default()
    });
    vm.recent_status = "Loading recent feeds...".into();
    vm.recent_has_more = true;
    vm.recent_loading = true;

    let snapshot = vm.recent_feeds_snapshot();

    assert_eq!(snapshot.display.heading, "Recent Feeds");
    assert_eq!(snapshot.display.load_more_button_id, "recent-load-more");
    assert_eq!(snapshot.display.empty_label, "No recent feeds");
    assert_eq!(snapshot.display.load_more_label, "Load more");
    assert_eq!(snapshot.feeds.len(), 1);
    assert_eq!(snapshot.status, "Loading recent feeds...");
    assert!(snapshot.has_more);
    assert!(snapshot.loading);
    assert!(!snapshot.empty);
}

#[test]
fn inspector_chrome_display_projects_breadcrumb_and_empty_state() {
    let display = SearchViewModel::inspector_chrome_display();
    assert_eq!(
        display.breadcrumb_root_button_id,
        "inspector-breadcrumb-root"
    );
    assert_eq!(display.scroll_id, "inspector-scroll");
    assert_eq!(display.breadcrumb_root_label, "Results");
    assert_eq!(display.empty_icon, "\u{1F50D}");
    assert_eq!(display.empty_label, "Select a result to inspect");
}

#[test]
fn inspector_status_messages_are_vm_owned() {
    assert_eq!(
        SearchViewModel::inspector_loading_message("Way to Go"),
        "Loading Way to Go..."
    );
    assert_eq!(
        SearchViewModel::inspector_error_message("offline"),
        "Error: offline"
    );
}

#[test]
fn deferred_panel_display_projects_heading_and_loading_labels() {
    let contributors = SearchViewModel::deferred_panel_display(DeferredPanelKind::Contributors);
    assert_eq!(contributors.section_id, "section:contributors");
    assert_eq!(contributors.heading_label, "Contributors");
    assert_eq!(contributors.loading_label, "Loading contributors...");
    assert_eq!(contributors.empty_label, "No contributors found");

    let value_routes = SearchViewModel::deferred_panel_display(DeferredPanelKind::ValueRoutes);
    assert_eq!(value_routes.section_id, "section:value-routes");
    assert_eq!(value_routes.heading_label, "Value Routes");
    assert_eq!(value_routes.loading_label, "Loading value routes...");
    assert_eq!(value_routes.empty_label, "No value routes found");
}

#[test]
fn deferred_panel_empty_line_projects_label() {
    assert_eq!(
        SearchViewModel::deferred_panel_empty_line("No value routes found"),
        "No value routes found"
    );
}

#[test]
fn feed_inspector_tracks_defaults_missing_tracks_to_empty_list() {
    let feed = Feed::default();
    assert!(SearchViewModel::feed_inspector_tracks(&feed).is_empty());

    let feed = Feed {
        tracks: Some(vec![Track {
            title: Some("Track".into()),
            ..Track::default()
        }]),
        ..Feed::default()
    };
    assert_eq!(SearchViewModel::feed_inspector_tracks(&feed).len(), 1);
}

#[test]
fn feed_list_section_display_projects_heading() {
    assert_eq!(
        SearchViewModel::feed_list_section_display(),
        SearchFeedListSectionDisplay { heading: "Feeds" }
    );
}

#[test]
fn inspector_title_display_projects_recents_root_and_frame_title() {
    assert_eq!(
        SearchViewModel::inspector_title_display(true, None),
        "Recent Feeds"
    );
    assert_eq!(
        SearchViewModel::inspector_title_display(false, Some("Way to Go")),
        "Way to Go"
    );
    assert_eq!(SearchViewModel::inspector_title_display(false, None), "");
}

#[test]
fn result_row_key_display_and_inspector_title_are_pure() {
    let row = ResultRow::new("feed", "feed-1", None);
    assert_eq!(row.key(), "index:feed:feed-1");
    let display = row.display();
    assert_eq!(display.element_id, "result-item:index:feed:feed-1");
    assert_eq!(display.kind_label, "feed");
    assert_eq!(display.line1, "feed-1");
    assert_eq!(row.inspector_title(), "feed-1");

    let item = row.render_item();
    assert_eq!(item.selection_key, "index:feed:feed-1");
    assert_eq!(item.display.element_id, "result-item:index:feed:feed-1");
    let (source, entity_type, entity_id, feed_guid, title) = item.navigation_target.into_parts();
    assert_eq!(
        (
            source,
            entity_type.as_str(),
            entity_id.as_str(),
            feed_guid.as_deref(),
            title.as_str()
        ),
        (
            SearchResultSource::MusicIndex,
            "feed",
            "feed-1",
            None,
            "feed-1"
        )
    );
}

#[test]
fn musicindex_track_result_row_preserves_feed_scope_for_navigation_identity() {
    let row = ResultRow::musicindex_track("track-1", Some("feed-1".into()), None);

    assert_eq!(row.key(), "index:track:feed-1:track-1");
    assert_eq!(
        row.display().element_id,
        "result-item:index:track:feed-1:track-1"
    );

    let (source, entity_type, entity_id, feed_guid, title) =
        row.render_item().navigation_target.into_parts();
    assert_eq!(source, SearchResultSource::MusicIndex);
    assert_eq!(entity_type, "track");
    assert_eq!(entity_id, "track-1");
    assert_eq!(feed_guid.as_deref(), Some("feed-1"));
    assert_eq!(title, "track-1");
}

#[test]
fn search_view_model_select_result_and_recent_feed_set_origin_and_key() {
    let mut vm = SearchViewModel::new();

    vm.select_result("track", "track-1");
    assert_eq!(vm.selected_key.as_deref(), Some("index:track:track-1"));
    assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Search));

    vm.select_recent_feed("feed-1");
    assert_eq!(vm.selected_key.as_deref(), Some("index:feed:feed-1"));
    assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Recents));
}

#[test]
fn search_view_model_navigation_targets_follow_selection_and_clamp_edges() {
    let mut vm = SearchViewModel::new();
    vm.results = vec![
        ResultRow::new(
            "feed",
            "feed-1",
            Some(EntityDetail::Feed(Feed {
                title: Some("First Feed".into()),
                ..Feed::default()
            })),
        ),
        ResultRow::new(
            "track",
            "track-1",
            Some(EntityDetail::Track(Track {
                name: Some("Second Track".into()),
                ..Track::default()
            })),
        ),
    ];

    let (source, entity_type, entity_id, feed_guid, title) = vm
        .next_result_target()
        .expect("first result should be selected when no row is selected")
        .into_parts();
    assert_eq!(
        (
            source,
            entity_type.as_str(),
            entity_id.as_str(),
            feed_guid.as_deref(),
            title.as_str()
        ),
        (
            SearchResultSource::MusicIndex,
            "feed",
            "feed-1",
            None,
            "First Feed"
        )
    );

    vm.select_result("track", "track-1");
    let (source, entity_type, entity_id, feed_guid, title) = vm
        .previous_result_target()
        .expect("previous result should move to first row")
        .into_parts();
    assert_eq!(
        (
            source,
            entity_type.as_str(),
            entity_id.as_str(),
            feed_guid.as_deref(),
            title.as_str()
        ),
        (
            SearchResultSource::MusicIndex,
            "feed",
            "feed-1",
            None,
            "First Feed"
        )
    );

    let (source, entity_type, entity_id, feed_guid, title) = vm
        .next_result_target()
        .expect("next result should clamp at the final row")
        .into_parts();
    assert_eq!(
        (
            source,
            entity_type.as_str(),
            entity_id.as_str(),
            feed_guid.as_deref(),
            title.as_str()
        ),
        (
            SearchResultSource::MusicIndex,
            "track",
            "track-1",
            None,
            "Second Track"
        )
    );
}

#[test]
fn search_view_model_endpoint_reset_clears_snapshots_and_marks_status() {
    let mut vm = SearchViewModel::new();
    vm.results.push(ResultRow::new("feed", "f1", None));
    vm.loading = true;
    vm.status = "Searching...".into();
    vm.cursor = Some("cursor".into());
    vm.has_more = true;
    vm.select("feed:f1");
    vm.mark_inspector_from_search();
    vm.recent_feeds.push(Feed::default());
    vm.recent_cursor = Some("recent".into());
    vm.recent_has_more = true;
    vm.recent_loaded_once = true;
    vm.recent_status = "Loaded".into();

    vm.reset_for_endpoint_change();

    assert!(vm.results.is_empty());
    assert!(!vm.loading);
    assert_eq!(vm.status, "MusicIndex endpoint updated");
    assert_eq!(vm.cursor, None);
    assert!(!vm.has_more);
    assert_eq!(vm.selected_key, None);
    assert_eq!(vm.inspector_origin, None);
    assert!(vm.recent_feeds.is_empty());
    assert_eq!(vm.recent_cursor, None);
    assert!(!vm.recent_has_more);
    assert!(!vm.recent_loaded_once);
    assert!(vm.recent_status.is_empty());
}

#[test]
fn search_view_model_return_to_recent_feeds_clears_search_pane() {
    let mut vm = SearchViewModel::new();
    vm.results.push(ResultRow::new("feed", "f1", None));
    vm.loading = true;
    vm.status = "Searching...".into();
    vm.cursor = Some("cursor".into());
    vm.has_more = true;
    vm.select("feed:f1");
    vm.mark_inspector_from_search();

    assert!(vm.return_to_recent_feeds());

    assert!(vm.results.is_empty());
    assert!(!vm.loading);
    assert!(vm.status.is_empty());
    assert_eq!(vm.cursor, None);
    assert!(!vm.has_more);
    assert_eq!(vm.selected_key, None);
    assert_eq!(vm.inspector_origin, None);

    vm.recent_loaded_once = true;
    assert!(!vm.return_to_recent_feeds());
}

#[test]
fn search_view_model_recent_feed_load_intent_tracks_append_cursor() {
    let mut vm = SearchViewModel::new();
    vm.recent_feeds.push(Feed::default());
    vm.recent_cursor = Some("next".into());
    vm.recent_has_more = true;

    let intent = vm
        .begin_recent_feed_load(true)
        .expect("idle VM should begin recent load");

    assert_eq!(intent.into_cursor().as_deref(), Some("next"));
    assert!(vm.recent_loading);
    assert_eq!(vm.recent_status, "Loading more recent feeds...");
    assert_eq!(vm.recent_feeds.len(), 1);
    assert!(vm.begin_recent_feed_load(true).is_none());
}

#[test]
fn search_view_model_recent_feed_fresh_load_resets_prior_page() {
    let mut vm = SearchViewModel::new();
    vm.recent_feeds.push(Feed::default());
    vm.recent_cursor = Some("next".into());
    vm.recent_has_more = true;

    let intent = vm
        .begin_recent_feed_load(false)
        .expect("idle VM should begin recent load");

    assert_eq!(intent.into_cursor(), None);
    assert!(vm.recent_feeds.is_empty());
    assert_eq!(vm.recent_cursor, None);
    assert!(!vm.recent_has_more);
    assert_eq!(vm.recent_status, "Loading recent feeds...");
}

#[test]
fn search_view_model_recent_feed_finish_and_fail_update_state() {
    let mut vm = SearchViewModel::new();
    assert!(vm.begin_recent_feed_load(false).is_some());

    vm.finish_recent_feed_load(api::RecentFeedsResponse {
        data: vec![Feed::default()],
        pagination: api::Pagination {
            has_more: true,
            cursor: Some("next".into()),
        },
    });

    assert!(!vm.recent_loading);
    assert!(vm.recent_loaded_once);
    assert_eq!(vm.recent_feeds.len(), 1);
    assert_eq!(vm.recent_cursor.as_deref(), Some("next"));
    assert!(vm.recent_has_more);
    assert!(vm.recent_status.is_empty());

    assert!(vm.begin_recent_feed_load(true).is_some());
    vm.fail_recent_feed_load("offline");

    assert!(!vm.recent_loading);
    assert!(vm.recent_loaded_once);
    assert_eq!(vm.recent_status, "Error: offline");
}

#[test]
fn normalized_search_query_rejects_non_search_terms() {
    assert_eq!(normalized_search_query("  feed  ").as_deref(), Some("feed"));
    assert_eq!(
        normalized_search_query("  c++ music  ").as_deref(),
        Some("c++ music")
    );
    assert_eq!(normalized_search_query(r"\"), None);
    assert_eq!(normalized_search_query("  ***  "), None);
    assert_eq!(normalized_search_query(" \n\t "), None);
}

#[test]
fn search_view_model_begin_search_load_sets_status_and_intent() {
    let mut vm = SearchViewModel::new();
    vm.set_type_filter(2);
    vm.fuzzy_search = false;
    vm.cursor = Some("next".into());
    vm.results.push(ResultRow::new("feed", "old", None));
    vm.select("feed:old");
    vm.mark_inspector_from_search();

    let intent = vm
        .begin_search_load(false)
        .expect("idle VM should begin a fresh search");

    assert_eq!(intent.type_filter(), 2);
    assert_eq!(intent.cursor(), None);
    assert!(!intent.fuzzy());
    assert!(vm.loading);
    assert_eq!(vm.status, "Searching...");
    assert!(vm.results.is_empty());
    assert_eq!(vm.cursor, None);
    assert_eq!(vm.selected_key, None);
    assert_eq!(vm.inspector_origin, None);
    assert!(vm.begin_search_load(false).is_none());
}

#[test]
fn search_view_model_begin_search_append_preserves_existing_results() {
    let mut vm = SearchViewModel::new();
    vm.cursor = Some("next".into());
    vm.results.push(ResultRow::new("feed", "old", None));

    let intent = vm
        .begin_search_load(true)
        .expect("idle VM should begin an append search");

    assert_eq!(intent.cursor(), Some("next"));
    assert_eq!(intent.type_filter(), 0);
    assert!(intent.fuzzy());
    assert_eq!(vm.status, "Loading more...");
    assert_eq!(vm.results.len(), 1);
}

#[test]
fn search_view_model_finish_search_load_formats_counts_and_cursor() {
    let mut vm = SearchViewModel::new();
    assert!(vm.begin_search_load(false).is_some());

    vm.finish_search_load(
        SearchBatch {
            rows: vec![ResultRow::new("feed", "f1", None)],
            has_more: true,
            cursor: Some("next".into()),
        },
        false,
    );

    assert!(!vm.loading);
    assert_eq!(vm.results.len(), 1);
    assert_eq!(vm.cursor.as_deref(), Some("next"));
    assert!(vm.has_more);
    assert_eq!(vm.status, "1 result+");
}

#[test]
fn search_view_model_finish_search_append_dedupes_existing_rows() {
    let mut vm = SearchViewModel::new();
    vm.results.push(ResultRow::new("feed", "f1", None));
    assert!(vm.begin_search_load(true).is_some());

    vm.finish_search_load(
        SearchBatch {
            rows: vec![
                ResultRow::new("feed", "f1", None),
                ResultRow::new("track", "t1", None),
            ],
            has_more: false,
            cursor: None,
        },
        true,
    );

    assert_eq!(vm.results.len(), 2);
    assert_eq!(vm.status, "2 results");
}

#[test]
fn search_view_model_finish_empty_fresh_search_clears_state() {
    let mut vm = SearchViewModel::new();
    assert!(vm.begin_search_load(false).is_some());

    vm.finish_search_load(
        SearchBatch {
            rows: Vec::new(),
            has_more: false,
            cursor: Some("ignored".into()),
        },
        false,
    );

    assert!(!vm.loading);
    assert!(vm.results.is_empty());
    assert_eq!(vm.cursor, None);
    assert!(!vm.has_more);
    assert!(vm.status.is_empty());
}

#[test]
fn search_view_model_fail_search_load_sets_error_status() {
    let mut vm = SearchViewModel::new();
    assert!(vm.begin_search_load(false).is_some());

    vm.fail_search_load("offline");

    assert!(!vm.loading);
    assert_eq!(vm.status, "Error: offline");
}

#[test]
fn search_view_model_merges_artist_result_detail_for_matching_result() {
    let mut vm = SearchViewModel::new();
    vm.results = vec![
        ResultRow::new(
            "artist",
            "artist-1",
            Some(EntityDetail::Artist(Artist {
                name: Some("Artist".into()),
                track_count: Some(1),
                feed_count: Some(1),
                image_url: Some("old.png".into()),
                ..Artist::default()
            })),
        ),
        ResultRow::new(
            "artist",
            "artist-2",
            Some(EntityDetail::Artist(Artist {
                track_count: Some(2),
                feed_count: Some(2),
                image_url: Some("keep.png".into()),
                ..Artist::default()
            })),
        ),
    ];

    vm.merge_artist_result_detail(
        "artist-1",
        &Artist {
            track_count: Some(10),
            feed_count: Some(3),
            image_url: Some("new.png".into()),
            ..Artist::default()
        },
    );

    let Some(EntityDetail::Artist(artist)) = &vm.results[0].detail else {
        panic!("expected artist detail");
    };
    assert_eq!(artist.track_count, Some(10));
    assert_eq!(artist.feed_count, Some(3));
    assert_eq!(artist.image_url.as_deref(), Some("new.png"));

    let Some(EntityDetail::Artist(other_artist)) = &vm.results[1].detail else {
        panic!("expected artist detail");
    };
    assert_eq!(other_artist.track_count, Some(2));
    assert_eq!(other_artist.feed_count, Some(2));
    assert_eq!(other_artist.image_url.as_deref(), Some("keep.png"));
}

#[test]
fn search_view_model_playlist_snapshot_and_failures_update_status() {
    let mut vm = SearchViewModel::new();
    let mut playlist = playlist("Focus");
    playlist.id = 12;

    vm.replace_playlists(vec![playlist]);
    assert_eq!(vm.playlists.len(), 1);

    vm.fail_playlist_load("db");
    assert_eq!(vm.status, "Error loading playlists: db");
    vm.fail_feed_subscription("offline");
    assert_eq!(vm.status, "Error subscribing feed: offline");
    vm.fail_feed_tracks_load("db");
    assert_eq!(vm.status, "Error loading feed tracks: db");
    vm.set_feed_has_no_tracks();
    assert_eq!(vm.status, "Feed has no tracks");
    vm.fail_playlist_create("exists");
    assert_eq!(vm.status, "Create playlist: exists");
    vm.set_track_not_in_library();
    assert_eq!(vm.status, "Track not in local library");
}

#[test]
fn search_view_model_playlist_append_intent_and_finish_format_status() {
    let mut vm = SearchViewModel::new();
    let mut playlist = playlist("Focus");
    playlist.id = 12;
    vm.replace_playlists(vec![playlist]);

    let intent = vm
        .begin_playlist_append(12, vec![7, 8])
        .expect("non-empty track ids should build an append intent");

    assert_eq!(intent.playlist_id(), 12);
    assert_eq!(intent.track_ids(), &[7, 8]);
    assert_eq!(intent.total_tracks(), 2);
    assert_eq!(intent.playlist_name(), "Focus");
    assert_eq!(vm.status, "Downloading 2 tracks...");

    vm.finish_playlist_append(&intent, PlaylistAppendOutcome::new(1, 1, 1));
    assert_eq!(vm.status, "Added 1 of 2 to Focus (downloaded 1); 1 failed");
}

#[test]
fn search_view_model_playlist_append_ignores_empty_and_formats_failure() {
    let mut vm = SearchViewModel::new();
    vm.status = "Ready".into();

    assert!(vm.begin_playlist_append(12, Vec::new()).is_none());
    assert_eq!(vm.status, "Ready");

    vm.fail_playlist_append("offline");
    assert_eq!(vm.status, "Error adding to playlist: offline");
}

#[test]
fn search_view_model_track_operation_rejects_empty_and_duplicate_keys() {
    let mut vm = SearchViewModel::new();

    assert!(!vm.begin_track_operation(""));
    assert!(!vm.is_track_operation_in_flight(""));
    assert!(vm.begin_track_operation("track:1"));
    assert!(vm.is_track_operation_in_flight("track:1"));
    assert!(!vm.begin_track_operation("track:1"));
}

#[test]
fn search_subscription_command_formats_begin_and_error_messages() {
    assert_eq!(
        SearchSubscriptionCommand::Download.begin_message(),
        "Downloading..."
    );
    assert_eq!(
        SearchSubscriptionCommand::track_download_success_message(),
        "Downloaded track"
    );
    assert_eq!(
        SearchSubscriptionCommand::Remove.begin_message(),
        "Removing..."
    );
    assert_eq!(
        SearchSubscriptionCommand::Download.error_message("offline"),
        "Download error: offline"
    );
    assert_eq!(
        SearchSubscriptionCommand::Remove.error_message("locked"),
        "Remove error: locked"
    );
    assert_eq!(
        SearchSubscriptionCommand::Download.success_message(0),
        "Downloaded track"
    );
    assert_eq!(
        SearchSubscriptionCommand::Download.success_message(2),
        "Downloaded track, applied 2 ID3 edits"
    );
    assert_eq!(
        SearchSubscriptionCommand::Remove.success_message(0),
        "Removed track"
    );
}

#[test]
fn search_view_model_finishes_track_download_and_remove_operations() {
    let mut vm = SearchViewModel::new();

    assert!(vm.begin_track_operation("track:1"));
    vm.finish_track_download("track:1", "Downloaded");
    assert!(!vm.is_track_operation_in_flight("track:1"));
    assert_eq!(vm.status, "Downloaded");

    assert!(vm.begin_track_operation("track:2"));
    vm.finish_track_remove("track:2", "Removed");
    assert!(!vm.is_track_operation_in_flight("track:2"));
    assert_eq!(vm.status, "Removed");
}

#[test]
fn search_view_model_fails_track_operations_with_contextual_status() {
    let mut vm = SearchViewModel::new();

    assert!(vm.begin_track_operation("track:1"));
    vm.fail_track_download("track:1", "offline");
    assert!(!vm.is_track_operation_in_flight("track:1"));
    assert_eq!(vm.status, "Download error: offline");

    assert!(vm.begin_track_operation("track:2"));
    vm.fail_track_remove("track:2", "locked");
    assert!(!vm.is_track_operation_in_flight("track:2"));
    assert_eq!(vm.status, "Remove error: locked");
}

#[test]
fn search_view_model_tracks_resize_lifecycle() {
    let mut vm = SearchViewModel::new();

    assert!(!vm.is_resizing());
    vm.begin_resize();
    assert!(vm.is_resizing());
    vm.end_resize();
    assert!(!vm.is_resizing());
}

#[test]
fn search_view_model_clamps_split_pane_width() {
    let mut vm = SearchViewModel::new();

    vm.resize_split_pane(120.0, 200.0, 800.0);
    assert_width_eq(vm.split_pane_width(), 200.0);

    vm.resize_split_pane(900.0, 200.0, 800.0);
    assert_width_eq(vm.split_pane_width(), 800.0);

    vm.resize_split_pane(420.0, 200.0, 800.0);
    assert_width_eq(vm.split_pane_width(), 420.0);
}
