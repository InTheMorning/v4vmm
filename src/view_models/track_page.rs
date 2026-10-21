//! The track page of the Library and of the Index (ADR 0084 Decision 4,
//! packet 023).
//!
//! Both origins build the track page through this one view model. The
//! origin selects the data source and the commands, not the page. A section
//! shows when its data exists. This module holds no renderer type.
//! `src/ui/shells/track_page.rs` renders the result.

#![warn(clippy::pedantic)]

use crate::api::Feed;
use crate::db;
use crate::metadata::{
    drop_placeholder_source_text, feed_nostr, feed_publication_pubdate, feed_website, TrackContext,
};
use crate::view_models::detail_selection::index_track_guid;
use crate::view_models::library::{
    description_line_count, DescriptionDisclosureDisplay, DescriptionState, LibraryTrackActionVm,
    LibraryViewModel,
};
use crate::view_models::named_feeds::NamedFeed;
use crate::view_models::track_detail::{
    LibraryTrackActionState, TrackDetailPageVm, TrackDetailSurfaceContext, TrackDetailVm,
    TrackNameLinkTarget, TrackPageAction, TrackPageActionDisplay,
};
use crate::view_models::workspace::TrackKey;
use crate::view_models::{
    playlist_option_displays, ActionStatusMessageDisplay, PlaylistOptionDisplayVm,
};
use crate::views::{FeedView, IdentityLinkFact, TrackView};

/// The label of the action row of a track page, for each origin
/// (decision 7).
const TRACK_PAGE_ACTIONS_A11Y_LABEL: &str = "Track actions";
/// The accessibility label of the "⋯" menu of a track page.
const TRACK_PAGE_MENU_A11Y_LABEL: &str = "More actions for this track";
/// The accessibility label of the description panel of a track page.
const TRACK_DESCRIPTION_A11Y_LABEL: &str = "Toggle track description";
/// The element id of the description panel of a Library track page.
const LIBRARY_DESCRIPTION_ID: &str = "library-track-description";
/// The element id of the container of an Index track page.
const INDEX_CONTAINER_ID: &str = "index-detail-track";

/// The facts of the feed of a track page, from each origin (ADR 0075
/// Decision B and packet 050). The feed identity section, the "Feed
/// publication date" row and "Copy feed URL" read only these facts.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TrackFeedFacts {
    /// The feed GUID. `None` when the feed states none.
    pub(crate) feed_guid: Option<String>,
    /// The feed title, trimmed. `None` when the feed has no real title.
    pub(crate) title: Option<String>,
    /// The feed URL, trimmed. `None` when the feed states none.
    pub(crate) feed_url: Option<String>,
    /// The website of the feed.
    pub(crate) website: Option<String>,
    /// The Nostr key of the feed.
    pub(crate) nostr_npub: Option<String>,
    /// The publication date of the feed, already formatted, with its
    /// provider: `"RSS"` or `"MusicIndex"`.
    pub(crate) publication_date: Option<(String, &'static str)>,
}

impl TrackFeedFacts {
    /// The facts of a Library feed record.
    #[must_use]
    pub(crate) fn from_api_feed(feed: &Feed) -> Self {
        Self {
            feed_guid: feed.feed_guid.clone(),
            title: source_text(feed.title.as_deref()),
            feed_url: source_text(feed.feed_url.as_deref()),
            website: feed_website(feed),
            nostr_npub: feed_nostr(feed),
            publication_date: feed_publication_pubdate(feed),
        }
    }

    /// The facts of an Index feed. `FeedView::from_api` already keeps the
    /// website and the Nostr key in `identity`, and the publication date
    /// in `published` and `published_source`.
    #[must_use]
    pub(crate) fn from_feed_view(feed: &FeedView) -> Self {
        Self {
            feed_guid: feed.feed_guid.clone(),
            title: feed.title.clone(),
            feed_url: feed.feed_url.clone(),
            website: website_from_identity_links(&feed.identity.source_links),
            nostr_npub: feed.identity.nostr_npub.clone(),
            publication_date: feed.published.clone().zip(feed.published_source),
        }
    }
}

/// A trimmed source text, with no placeholder text. `FeedView::from_api`
/// uses the same rule for the title and the feed URL.
fn source_text(value: Option<&str>) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    drop_placeholder_source_text(value.map(str::to_owned))
}

/// The website of a feed from its identity links. This is the rule of
/// `metadata::feed_website`: the first link of type "website" or
/// "`web_page`".
fn website_from_identity_links(links: &[IdentityLinkFact]) -> Option<String> {
    links.iter().find_map(|link| {
        let link_type = link.link_type.as_deref()?;
        if link_type == "website" || link_type == "web_page" {
            link.url.clone()
        } else {
            None
        }
    })
}

/// The Library facts of a track page. The host copies them from its own
/// state. The view model names no Library state type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryTrackPageState {
    /// The Library track id.
    pub(crate) track_id: i64,
    /// The Library feed id of the album of the track.
    pub(crate) album_feed_id: i64,
    /// The title that the Library gives the page.
    pub(crate) title: String,
    /// The Library state that selects the page actions.
    pub(crate) action_state: LibraryTrackActionState,
    /// The download or removal message of the track. `None` when the
    /// track has no message.
    pub(crate) subscription_message: Option<String>,
    /// Why a `MusicBrainz` lookup cannot start. `None` when it can.
    pub(crate) musicbrainz_reason: Option<&'static str>,
    /// The element id of the page container.
    pub(crate) container_id: &'static str,
}

/// The origin of a track page, with the facts that select its commands.
#[derive(Clone, Debug, Eq, PartialEq)]
enum TrackPageOrigin {
    /// A Library track.
    Library(LibraryTrackPageState),
    /// An Index track that the Library does not hold.
    Index,
}

/// The origin-neutral input of a track page.
#[derive(Clone, Debug)]
pub(crate) struct TrackPageFacts {
    track: TrackView,
    origin: TrackPageOrigin,
    named_feeds: Vec<NamedFeed>,
    feed: Option<TrackFeedFacts>,
    /// The disclosure state of the description. `None` gives the state
    /// that the length of the text selects.
    description_state: Option<DescriptionState>,
    playlists: Vec<PlaylistOptionDisplayVm>,
}

impl TrackPageFacts {
    /// The facts of a Library track. The query layer already read the
    /// track and its feed. This function sends no request.
    #[must_use]
    pub(crate) fn from_library_track(context: &TrackContext, state: LibraryTrackPageState) -> Self {
        Self {
            track: TrackView::from_api(context.track.clone()),
            origin: TrackPageOrigin::Library(state),
            named_feeds: Vec::new(),
            feed: context.feed.as_ref().map(TrackFeedFacts::from_api_feed),
            description_state: None,
            playlists: Vec::new(),
        }
    }

    /// The facts of an Index track. `feed` is the `MusicIndex` feed of the
    /// track. It is `None` while the feed read runs, or after it fails.
    #[must_use]
    pub(crate) fn from_index_track(track: TrackView, feed: Option<&FeedView>) -> Self {
        let named_feeds = track.named_feeds.clone();
        Self {
            track,
            origin: TrackPageOrigin::Index,
            named_feeds,
            feed: feed.map(TrackFeedFacts::from_feed_view),
            description_state: None,
            playlists: Vec::new(),
        }
    }

    /// Sets the named feeds of the album feed of the track (ADR 0085
    /// Decisions 5 and 6).
    #[must_use]
    pub(crate) fn with_named_feeds(mut self, named_feeds: Vec<NamedFeed>) -> Self {
        self.named_feeds = named_feeds;
        self
    }

    /// Sets the disclosure state of the description. Each host keeps its
    /// own state store.
    #[must_use]
    pub(crate) fn with_description_state(mut self, state: DescriptionState) -> Self {
        self.description_state = Some(state);
        self
    }

    /// Sets the playlists of the "Add to playlist" popover.
    #[must_use]
    pub(crate) fn with_playlists(mut self, playlists: &[db::Playlist]) -> Self {
        self.playlists = playlist_option_displays(playlists);
        self
    }
}

/// The key of the description state of an Index track. It is the Index
/// activation target of the track, built the same way as the click key of
/// a track row.
#[must_use]
pub(crate) fn index_track_description_key(track: &TrackView) -> String {
    TrackKey::from_index_track_target(
        track.feed_guid.as_deref(),
        index_track_guid(track).unwrap_or_default(),
    )
    .index_target()
}

/// One command of a track page. The host runs it on the data that it holds
/// at the time of the click.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TrackPageCommand {
    /// Run a page action.
    Action(TrackPageAction),
    /// Add the track to the playlist with this id.
    AddToPlaylist(i64),
    /// Make a playlist with this name, and add the track to it.
    CreatePlaylist(String),
    /// Open or close the description.
    ToggleDescription,
    /// Open the page of a name link.
    OpenNameLink(TrackNameLinkTarget),
}

/// One display-ready action of a track page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackPageActionVm {
    pub(crate) action: TrackPageAction,
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
    pub(crate) available: bool,
    pub(crate) destructive: bool,
    /// The text under an unavailable menu item that tells why. `None`
    /// when the item has no such text.
    pub(crate) reason: Option<&'static str>,
}

/// The action row of a track page (ADR 0083 Decision 5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackPageActionRowVm {
    /// The one filled button.
    pub(crate) filled: TrackPageActionVm,
    /// The plain buttons.
    pub(crate) plain: Vec<TrackPageActionVm>,
    /// The items of the "⋯" menu. A destructive item is last.
    pub(crate) menu: Vec<TrackPageActionVm>,
    /// The element id of the "⋯" menu.
    pub(crate) menu_id: String,
    /// The accessibility label of the "⋯" menu.
    pub(crate) menu_a11y_label: &'static str,
    /// The accessibility label of the action row.
    pub(crate) a11y_label: &'static str,
    /// The status message below the row. `None` when no message exists.
    pub(crate) message: Option<ActionStatusMessageDisplay>,
    /// The element id of the playlist popover. `None` when the track
    /// cannot go into a playlist.
    pub(crate) playlist_popover_id: Option<String>,
    /// The playlists of the popover.
    pub(crate) playlists: Vec<PlaylistOptionDisplayVm>,
}

/// The description panel of a track page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackPageDescriptionVm {
    /// The element id of the panel.
    pub(crate) id: String,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) display: DescriptionDisclosureDisplay,
}

/// The track page: the shared surface and the parts that differ by origin.
#[derive(Debug)]
pub(crate) struct TrackPageVm<'a> {
    pub(crate) page: TrackDetailPageVm<'a>,
    pub(crate) actions: TrackPageActionRowVm,
    pub(crate) description: Option<TrackPageDescriptionVm>,
    /// The page shows "Inspect sources" (ADR 0083 Decision 8). Only a
    /// page with Library inspection facts shows it (decision 9).
    pub(crate) inspect_sources: bool,
    /// The element id of the page container.
    pub(crate) container_id: &'static str,
}

/// Builds the track page of `facts`.
#[must_use]
pub(crate) fn track_page_vm(facts: &TrackPageFacts) -> TrackPageVm<'_> {
    let context = match &facts.origin {
        TrackPageOrigin::Library(_) => TrackDetailSurfaceContext::Library,
        TrackPageOrigin::Index => TrackDetailSurfaceContext::Discover,
    };
    let mut detail = TrackDetailVm::new(&facts.track, context)
        .with_named_feeds(&facts.named_feeds)
        .with_feed_identity(facts.feed.as_ref());
    if let TrackPageOrigin::Library(state) = &facts.origin {
        detail = detail
            .with_override_title(Some(&state.title))
            .with_album_feed_id(state.album_feed_id)
            .with_library_state(state.action_state);
    }
    let actions = track_page_action_row(facts, &detail);
    let description = track_page_description(facts, &detail);
    let (inspect_sources, container_id) = match &facts.origin {
        TrackPageOrigin::Library(state) => (true, state.container_id),
        TrackPageOrigin::Index => (false, INDEX_CONTAINER_ID),
    };
    TrackPageVm {
        page: detail.page(),
        actions,
        description,
        inspect_sources,
        container_id,
    }
}

/// The action row of a track page. Each element id stays the id of its
/// origin (decision 16).
fn track_page_action_row(
    facts: &TrackPageFacts,
    detail: &TrackDetailVm<'_>,
) -> TrackPageActionRowVm {
    let actions = detail.page_actions();
    match &facts.origin {
        TrackPageOrigin::Library(state) => {
            let action_vm = |display: TrackPageActionDisplay, place: &str| {
                let reason = (display.action == TrackPageAction::MusicBrainzLookup
                    && !display.available)
                    .then_some(state.musicbrainz_reason)
                    .flatten();
                track_page_action_vm(display, &format!("track-page-{place}"), reason)
            };
            let message = LibraryTrackActionVm::new(state.subscription_message.as_deref())
                .subscription_message_display();
            TrackPageActionRowVm {
                filled: action_vm(actions.filled, "action"),
                plain: actions
                    .plain
                    .iter()
                    .map(|display| action_vm(*display, "action"))
                    .collect(),
                menu: actions
                    .menu
                    .iter()
                    .map(|display| action_vm(*display, "menu"))
                    .collect(),
                menu_id: format!("track-page-menu-{}", state.track_id),
                menu_a11y_label: TRACK_PAGE_MENU_A11Y_LABEL,
                a11y_label: TRACK_PAGE_ACTIONS_A11Y_LABEL,
                message,
                playlist_popover_id: Some(
                    LibraryTrackActionVm::playlist_display(state.track_id).popover_id,
                ),
                playlists: facts.playlists.clone(),
            }
        }
        TrackPageOrigin::Index => {
            let feed_guid = facts
                .track
                .feed_guid
                .as_deref()
                .map(str::trim)
                .filter(|feed_guid| !feed_guid.is_empty());
            let action_vm = |display: TrackPageActionDisplay, place: &str| {
                track_page_action_vm(display, &format!("index-track-page-{place}"), None)
            };
            // An Index track with no feed GUID has no menu: each menu
            // command reads the album feed by its GUID.
            let menu = match feed_guid {
                Some(_) => actions
                    .menu
                    .iter()
                    .map(|display| action_vm(*display, "menu"))
                    .collect(),
                None => Vec::new(),
            };
            TrackPageActionRowVm {
                filled: action_vm(actions.filled, "action"),
                plain: actions
                    .plain
                    .iter()
                    .map(|display| action_vm(*display, "action"))
                    .collect(),
                menu,
                menu_id: format!("index-track-page-menu-{}", feed_guid.unwrap_or("unknown")),
                menu_a11y_label: TRACK_PAGE_MENU_A11Y_LABEL,
                a11y_label: TRACK_PAGE_ACTIONS_A11Y_LABEL,
                message: None,
                playlist_popover_id: None,
                playlists: Vec::new(),
            }
        }
    }
}

/// One action. The page knows no release kind, so the label says
/// "Download release" (ADR 0084 Decision 5).
fn track_page_action_vm(
    display: TrackPageActionDisplay,
    id_prefix: &str,
    reason: Option<&'static str>,
) -> TrackPageActionVm {
    let action = display.action;
    TrackPageActionVm {
        action,
        id: format!("{id_prefix}:{action:?}"),
        label: action.label(None),
        a11y_label: action.a11y_label(None),
        available: display.available,
        destructive: action.is_destructive(),
        reason,
    }
}

/// The description panel of a track page: one text rule and one
/// projection for each origin (decision 10). `None` when the track has no
/// description.
fn track_page_description(
    facts: &TrackPageFacts,
    detail: &TrackDetailVm<'_>,
) -> Option<TrackPageDescriptionVm> {
    let description = detail.description();
    let text = LibraryViewModel::display_description_text(description.as_deref())?;
    let state = facts
        .description_state
        .unwrap_or_else(|| DescriptionState::project(description_line_count(Some(text))));
    let id = match &facts.origin {
        TrackPageOrigin::Library(_) => LIBRARY_DESCRIPTION_ID.to_owned(),
        TrackPageOrigin::Index => format!(
            "index-track-description:{}",
            index_track_description_key(&facts.track)
        ),
    };
    Some(TrackPageDescriptionVm {
        id,
        label: detail.labels().description_label(),
        a11y_label: TRACK_DESCRIPTION_A11Y_LABEL,
        display: DescriptionDisclosureDisplay::project(text, state),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{SourceEntityId, SourceEntityLink};

    /// One recorded feed with a website, a Nostr key, a channel `pubDate`
    /// and a feed URL.
    fn recorded_feed() -> Feed {
        Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("MoeFactz".into()),
            feed_url: Some("https://example.test/feed.xml".into()),
            channel_pub_date: Some(1_758_369_600),
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
        }
    }

    /// R23-01: the Library feed and the Index feed give equal facts for
    /// one feed.
    #[test]
    fn adr_0084_track_page_r23_01_both_feed_sources_give_equal_facts() {
        let feed = recorded_feed();
        let library = TrackFeedFacts::from_api_feed(&feed);
        let index = TrackFeedFacts::from_feed_view(&FeedView::from_api(feed));

        assert_eq!(library, index);
        assert_eq!(
            library.website.as_deref(),
            Some("https://example.test/feed")
        );
        assert_eq!(library.nostr_npub.as_deref(), Some("npub1feed"));
        assert_eq!(
            library.publication_date,
            Some(("Sep 20, 2025".to_string(), "RSS"))
        );
        assert_eq!(
            library.feed_url.as_deref(),
            Some("https://example.test/feed.xml")
        );
    }

    /// R23-01: a "`web_page`" link gives the same website from each source.
    #[test]
    fn adr_0084_track_page_r23_01_web_page_link_gives_equal_website() {
        let mut feed = recorded_feed();
        feed.source_links = Some(vec![SourceEntityLink {
            link_type: Some("web_page".into()),
            url: Some("https://example.test/page".into()),
            ..Default::default()
        }]);
        let library = TrackFeedFacts::from_api_feed(&feed);
        let index = TrackFeedFacts::from_feed_view(&FeedView::from_api(feed));

        assert_eq!(library, index);
        assert_eq!(
            library.website.as_deref(),
            Some("https://example.test/page")
        );
    }

    use crate::api::Track;
    use crate::metadata::TrackContext;
    use crate::view_models::track_detail::TrackDetailSummaryRow;

    const TRACK_ID: i64 = 42;
    const ALBUM_FEED_ID: i64 = 7;

    /// One recorded track of the recorded feed, with no date of its own.
    fn recorded_track() -> Track {
        Track {
            track_guid: Some("track-guid".into()),
            feed_guid: Some("feed-guid".into()),
            feed_title: Some("MoeFactz".into()),
            title: Some("Song".into()),
            duration_secs: Some(125),
            track_number: Some(3),
            description: Some("A track description".into()),
            ..Default::default()
        }
    }

    fn library_state(action_state: LibraryTrackActionState) -> LibraryTrackPageState {
        LibraryTrackPageState {
            track_id: TRACK_ID,
            album_feed_id: ALBUM_FEED_ID,
            title: "Song".into(),
            action_state,
            subscription_message: None,
            musicbrainz_reason: Some("Download the track first"),
            container_id: "track-detail-scroll",
        }
    }

    fn library_facts(feed: Option<Feed>) -> TrackPageFacts {
        let context = TrackContext::new(recorded_track(), feed);
        TrackPageFacts::from_library_track(
            &context,
            library_state(LibraryTrackActionState::default()),
        )
    }

    fn index_facts(feed: Option<&Feed>) -> TrackPageFacts {
        let feed = feed.cloned().map(FeedView::from_api);
        TrackPageFacts::from_index_track(TrackView::from_api(recorded_track()), feed.as_ref())
    }

    fn summary_labels(rows: &[TrackDetailSummaryRow]) -> Vec<&str> {
        rows.iter().map(|row| row.label.as_str()).collect()
    }

    /// R23-02: the defect fix. An Index track page with feed facts gives
    /// the feed identity section. It names the feed title and holds the
    /// feed website and the feed Nostr key.
    #[test]
    fn adr_0084_track_page_r23_02_index_track_shows_the_feed_identity_section() {
        let feed = recorded_feed();
        let facts = index_facts(Some(&feed));
        let vm = track_page_vm(&facts);

        let section = vm
            .page
            .feed_identity_section()
            .expect("an Index track with feed facts has a feed identity section");
        assert_eq!(section.owner_label, "MoeFactz");
        assert_eq!(
            section
                .actions
                .iter()
                .map(|action| action.payload.clone())
                .collect::<Vec<_>>(),
            vec![
                Some("https://example.test/feed".to_string()),
                Some("npub1feed".to_string()),
            ]
        );
    }

    /// R23-03: the defect fix. An Index track with no date of its own shows
    /// "Feed publication date". A track with its own date has no such row.
    #[test]
    fn adr_0084_track_page_r23_03_index_track_shows_the_feed_publication_date() {
        let feed = recorded_feed();
        let facts = index_facts(Some(&feed));
        let vm = track_page_vm(&facts);
        let rows = vm.page.detail().summary_rows();
        assert!(rows
            .iter()
            .any(|row| row.label == "Feed publication date" && row.value == "Sep 20, 2025 (RSS)"));

        let mut dated = recorded_track();
        dated.pub_date = Some(1_712_275_200);
        let feed_view = FeedView::from_api(feed);
        let facts = TrackPageFacts::from_index_track(TrackView::from_api(dated), Some(&feed_view));
        let vm = track_page_vm(&facts);
        assert!(
            !summary_labels(&vm.page.detail().summary_rows()).contains(&"Feed publication date")
        );
    }

    /// R23-04: an Index track with no feed facts, while the read runs or
    /// after it fails, gives no feed section and no "Feed publication
    /// date" row. The other sections stay the same.
    #[test]
    fn adr_0084_track_page_r23_04_index_track_without_feed_facts_keeps_its_page() {
        let feed = recorded_feed();
        let with_facts = index_facts(Some(&feed));
        let without_facts = index_facts(None);
        let with_vm = track_page_vm(&with_facts);
        let without_vm = track_page_vm(&without_facts);

        assert!(without_vm.page.feed_identity_section().is_none());
        let without_rows = without_vm.page.detail().summary_rows();
        assert!(!summary_labels(&without_rows).contains(&"Feed publication date"));
        let with_rows: Vec<_> = with_vm
            .page
            .detail()
            .summary_rows()
            .into_iter()
            .filter(|row| row.label != "Feed publication date")
            .collect();
        assert_eq!(with_rows, without_rows);
        assert_eq!(
            with_vm.page.detail().display_title(),
            without_vm.page.detail().display_title()
        );
        assert_eq!(
            with_vm.page.detail().name_links(),
            without_vm.page.detail().name_links()
        );
        assert_eq!(
            with_vm.page.identity_actions(),
            without_vm.page.identity_actions()
        );
        assert_eq!(
            with_vm.page.detail().credits(),
            without_vm.page.detail().credits()
        );
        assert_eq!(with_vm.description, without_vm.description);
        assert_eq!(
            with_vm.actions.filled.action,
            without_vm.actions.filled.action
        );
    }

    /// R23-06: a Library track and the same track as an Index track, with
    /// the same feed, give an equal page. Only the actions, the ids, the
    /// link targets and `inspect_sources` differ.
    #[test]
    fn adr_0084_track_page_r23_06_both_origins_give_an_equal_page() {
        let feed = recorded_feed();
        let library = library_facts(Some(feed.clone()));
        let index = index_facts(Some(&feed));
        let library_vm = track_page_vm(&library);
        let index_vm = track_page_vm(&index);
        let (library_page, index_page) = (library_vm.page.detail(), index_vm.page.detail());

        assert_eq!(library_page.display_title(), index_page.display_title());
        assert_eq!(library_page.display_artist(), index_page.display_artist());
        let labels = |links: Vec<crate::view_models::track_detail::TrackNameLinkVm>| {
            links
                .into_iter()
                .map(|link| (link.label, link.a11y_label, link.note))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            labels(library_page.name_links()),
            labels(index_page.name_links())
        );
        assert_eq!(
            library_vm.page.identity_actions(),
            index_vm.page.identity_actions()
        );
        assert_eq!(library_page.summary_rows(), index_page.summary_rows());
        assert_eq!(library_page.credits(), index_page.credits());
        let library_section = library_vm.page.feed_identity_section();
        assert!(library_section.is_some());
        assert_eq!(library_section, index_vm.page.feed_identity_section());
        let library_description = library_vm.description.expect("a description");
        let index_description = index_vm.description.expect("a description");
        assert_eq!(library_description.display, index_description.display);
        assert_eq!(library_description.label, index_description.label);

        assert!(library_vm.inspect_sources);
        assert!(!index_vm.inspect_sources);
        assert_ne!(library_description.id, index_description.id);
        assert_ne!(library_vm.actions, index_vm.actions);
    }

    /// R23-07: for each Library state, the Library action row gives the
    /// filled action, the plain actions, the menu, each element id and the
    /// status message of the Library track page before packet 023.
    #[test]
    fn adr_0084_track_page_r23_07_library_action_row_keeps_each_state() {
        let feed = recorded_feed();
        let feed_facts = TrackFeedFacts::from_api_feed(&feed);
        let track_view = TrackView::from_api(recorded_track());
        for bits in 0_u8..16 {
            let action_state = LibraryTrackActionState {
                downloaded: bits & 1 != 0,
                subscription_busy: bits & 2 != 0,
                musicbrainz_available: bits & 4 != 0,
                release_download_active: bits & 8 != 0,
            };
            let mut state = library_state(action_state);
            state.subscription_message = action_state
                .subscription_busy
                .then(|| "Downloading...".to_string());
            let context = TrackContext::new(recorded_track(), Some(feed.clone()));
            let facts = TrackPageFacts::from_library_track(&context, state.clone());
            let row = track_page_vm(&facts).actions;

            let expected = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Library)
                .with_feed_identity(Some(&feed_facts))
                .with_library_state(action_state)
                .page_actions();
            let ids = |displays: &[TrackPageActionDisplay], place: &str| {
                displays
                    .iter()
                    .map(|display| {
                        (
                            display.action,
                            display.available,
                            format!("track-page-{place}:{:?}", display.action),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let actual = |actions: &[TrackPageActionVm]| {
                actions
                    .iter()
                    .map(|action| (action.action, action.available, action.id.clone()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                actual(std::slice::from_ref(&row.filled)),
                ids(std::slice::from_ref(&expected.filled), "action"),
                "filled action of {action_state:?}"
            );
            assert_eq!(actual(&row.plain), ids(&expected.plain, "action"));
            assert_eq!(actual(&row.menu), ids(&expected.menu, "menu"));
            assert_eq!(row.menu_id, format!("track-page-menu-{TRACK_ID}"));
            assert_eq!(row.a11y_label, "Track actions");
            assert_eq!(row.menu_a11y_label, "More actions for this track");
            assert_eq!(
                row.playlist_popover_id.as_deref(),
                Some(format!("track-inspector-add:{TRACK_ID}").as_str())
            );
            assert_eq!(
                row.message,
                LibraryTrackActionVm::new(state.subscription_message.as_deref())
                    .subscription_message_display()
            );
            assert_eq!(row.message.is_some(), action_state.subscription_busy);
            for item in &row.menu {
                let has_reason =
                    item.action == TrackPageAction::MusicBrainzLookup && !item.available;
                assert_eq!(item.reason.is_some(), has_reason);
            }
        }
    }

    /// R23-08: the Index action row gives the filled "Download album", no
    /// plain action, no message and the ids of the Index track page. With
    /// no feed GUID, the filled action is unavailable and the menu is
    /// empty.
    #[test]
    fn adr_0084_track_page_r23_08_index_action_row_keeps_its_ids() {
        let facts = index_facts(None);
        let row = track_page_vm(&facts).actions;
        assert_eq!(row.filled.action, TrackPageAction::DownloadAlbum);
        assert!(row.filled.available);
        assert_eq!(row.filled.id, "index-track-page-action:DownloadAlbum");
        assert_eq!(row.filled.label, "Download release");
        assert!(row.plain.is_empty());
        assert!(row.message.is_none());
        assert!(row.playlist_popover_id.is_none());
        assert_eq!(row.menu_id, "index-track-page-menu-feed-guid");
        assert_eq!(
            row.menu
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["index-track-page-menu:CopyFeedUrl"]
        );
        assert_eq!(row.a11y_label, "Track actions");

        let mut no_guid = recorded_track();
        no_guid.feed_guid = None;
        let feed = recorded_feed();
        let feed_view = FeedView::from_api(feed);
        let facts =
            TrackPageFacts::from_index_track(TrackView::from_api(no_guid), Some(&feed_view));
        let row = track_page_vm(&facts).actions;
        assert_eq!(row.filled.action, TrackPageAction::DownloadAlbum);
        assert!(!row.filled.available);
        assert!(row.menu.is_empty());
    }

    /// R23-09: `inspect_sources` is true for Library facts and false for
    /// Index facts.
    #[test]
    fn adr_0084_track_page_r23_09_only_library_facts_inspect_sources() {
        assert!(track_page_vm(&library_facts(None)).inspect_sources);
        assert!(!track_page_vm(&index_facts(None)).inspect_sources);
    }

    /// R23-10: the album name link is a data rule. A Library album feed id
    /// gives `LibraryAlbum`. Else the feed GUID gives `IndexAlbum`. The
    /// context does not select it.
    #[test]
    fn adr_0084_track_page_r23_10_album_link_target_is_a_data_rule() {
        let track = TrackView::from_api(recorded_track());
        for context in [
            TrackDetailSurfaceContext::Library,
            TrackDetailSurfaceContext::Discover,
        ] {
            let with_id = TrackDetailVm::new(&track, context).with_album_feed_id(ALBUM_FEED_ID);
            assert_eq!(
                with_id.name_links()[0].target,
                TrackNameLinkTarget::LibraryAlbum(ALBUM_FEED_ID),
                "{context:?} with a Library album feed id"
            );
            let without_id = TrackDetailVm::new(&track, context);
            assert_eq!(
                without_id.name_links()[0].target,
                TrackNameLinkTarget::IndexAlbum {
                    feed_guid: "feed-guid".to_string()
                },
                "{context:?} with no Library album feed id"
            );
        }
        let library = library_facts(None);
        assert_eq!(
            track_page_vm(&library).page.detail().name_links()[0].target,
            TrackNameLinkTarget::LibraryAlbum(ALBUM_FEED_ID)
        );
        let index = index_facts(None);
        assert_eq!(
            track_page_vm(&index).page.detail().name_links()[0].target,
            TrackNameLinkTarget::IndexAlbum {
                feed_guid: "feed-guid".to_string()
            }
        );
    }
}
