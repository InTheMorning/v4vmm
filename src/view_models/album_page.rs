//! The actions and the name links of an album page (ADR 0083 Decision 5,
//! task 005).
//!
//! The Library album page and the Index album page use this one model
//! (ADR 0037). The screen maps each action to its command.

#![warn(clippy::pedantic)]

use crate::view_models::entity_detail::ReleaseMembershipState;

/// The label of a publisher link. ADR 0077 Decision 6: `publisher_text` is
/// the feed owner, so a publisher link does not show it.
pub const PUBLISHER_LINK_LABEL: &str = "Publisher";
/// The accessibility label of the "⋯" menu of an album page.
pub const ALBUM_PAGE_MENU_A11Y_LABEL: &str = "More actions for this album";

/// One action of an album page.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AlbumPageAction {
    /// Download the album feed.
    DownloadAlbum,
    /// Add the Library tracks of the album to a playlist.
    AddToPlaylist,
    /// Copy the feed URL of the album.
    CopyFeedUrl,
    /// Look up the missing `MusicBrainz` fields of the album.
    MusicBrainzLookup,
    /// Remove the album from the Library, after a confirmation.
    RemoveAlbum,
}

impl AlbumPageAction {
    /// The visible label. A label that opens a confirmation ends with "…".
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DownloadAlbum => "Download album",
            Self::AddToPlaylist => "Add to playlist",
            Self::CopyFeedUrl => "Copy feed URL",
            Self::MusicBrainzLookup => "MusicBrainz lookup",
            Self::RemoveAlbum => "Remove album…",
        }
    }

    /// The accessibility label.
    #[must_use]
    pub const fn a11y_label(self) -> &'static str {
        match self {
            Self::DownloadAlbum => "Download this album",
            Self::AddToPlaylist => "Add this album to a playlist",
            Self::CopyFeedUrl => "Copy the feed URL of this album",
            Self::MusicBrainzLookup => "Look up missing MusicBrainz fields for this album",
            Self::RemoveAlbum => "Remove this album from the Library",
        }
    }

    /// A destructive action is last in its menu and asks for confirmation.
    #[must_use]
    pub const fn is_destructive(self) -> bool {
        matches!(self, Self::RemoveAlbum)
    }
}

/// A display-ready album page action with its typed availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlbumPageActionDisplay {
    /// The action.
    pub action: AlbumPageAction,
    /// The action can run now.
    pub available: bool,
}

impl AlbumPageActionDisplay {
    /// Make a display for `action`.
    #[must_use]
    pub const fn new(action: AlbumPageAction, available: bool) -> Self {
        Self { action, available }
    }
}

/// The actions of an album page in the order of ADR 0083 Decision 5: one
/// filled button, the plain buttons, and the "⋯" menu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlbumPageActions {
    /// The one filled button.
    pub filled: AlbumPageActionDisplay,
    /// The plain buttons.
    pub plain: Vec<AlbumPageActionDisplay>,
    /// The "⋯" menu items. A destructive item is last.
    pub menu: Vec<AlbumPageActionDisplay>,
}

/// The origin and the state of an album page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlbumPageOrigin {
    /// An album that the Index page shows.
    Index,
    /// A Library album.
    Library {
        /// The membership of the album in the Library.
        membership: ReleaseMembershipState,
        /// A `MusicBrainz` lookup can start now.
        musicbrainz_available: bool,
    },
}

/// The actions of an album page. "Copy feed URL" shows when the album has a
/// feed URL.
#[must_use]
pub fn album_page_actions(origin: AlbumPageOrigin, feed_url: Option<&str>) -> AlbumPageActions {
    let mut menu: Vec<_> = feed_url
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(|_| AlbumPageActionDisplay::new(AlbumPageAction::CopyFeedUrl, true))
        .into_iter()
        .collect();
    let AlbumPageOrigin::Library {
        membership,
        musicbrainz_available,
    } = origin
    else {
        return AlbumPageActions {
            filled: AlbumPageActionDisplay::new(AlbumPageAction::DownloadAlbum, true),
            plain: Vec::new(),
            menu,
        };
    };
    menu.push(AlbumPageActionDisplay::new(
        AlbumPageAction::MusicBrainzLookup,
        musicbrainz_available,
    ));
    let filled = match membership {
        ReleaseMembershipState::RemoteOnly => {
            AlbumPageActionDisplay::new(AlbumPageAction::DownloadAlbum, true)
        }
        ReleaseMembershipState::Downloading => {
            AlbumPageActionDisplay::new(AlbumPageAction::DownloadAlbum, false)
        }
        ReleaseMembershipState::InLibrary => {
            AlbumPageActionDisplay::new(AlbumPageAction::AddToPlaylist, true)
        }
        ReleaseMembershipState::Removing => {
            AlbumPageActionDisplay::new(AlbumPageAction::AddToPlaylist, false)
        }
    };
    match membership {
        ReleaseMembershipState::InLibrary => menu.push(AlbumPageActionDisplay::new(
            AlbumPageAction::RemoveAlbum,
            true,
        )),
        ReleaseMembershipState::Removing => menu.push(AlbumPageActionDisplay::new(
            AlbumPageAction::RemoveAlbum,
            false,
        )),
        ReleaseMembershipState::RemoteOnly | ReleaseMembershipState::Downloading => {}
    }
    AlbumPageActions {
        filled,
        plain: Vec::new(),
        menu,
    }
}

/// The page that a name link under the album title opens.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlbumNameLinkTarget {
    /// The artist page of a name. The Library opens its artist page. The
    /// Index opens the tracks that match the name.
    Artist(String),
    /// The publisher page of a publisher feed GUID (ADR 0077 Decision 1).
    Publisher(String),
}

/// A name under the album title that links to a page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlbumNameLinkVm {
    /// The visible name.
    pub label: String,
    /// The accessibility label.
    pub a11y_label: String,
    /// The page that the link opens.
    pub target: AlbumNameLinkTarget,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions_of(display: &[AlbumPageActionDisplay]) -> Vec<AlbumPageAction> {
        display.iter().map(|item| item.action).collect()
    }

    fn library(membership: ReleaseMembershipState) -> AlbumPageOrigin {
        AlbumPageOrigin::Library {
            membership,
            musicbrainz_available: true,
        }
    }

    const FEED_URL: Option<&str> = Some("https://feeds.example.test/album.xml");

    /// R83-51: each album state gives one filled action, no plain action,
    /// and the "⋯" items of the task 005 table.
    #[test]
    fn adr_0083_album_page_actions_follow_the_state_table() {
        let index = album_page_actions(AlbumPageOrigin::Index, FEED_URL);
        assert_eq!(index.filled.action, AlbumPageAction::DownloadAlbum);
        assert!(index.filled.available);
        assert!(index.plain.is_empty());
        assert_eq!(actions_of(&index.menu), vec![AlbumPageAction::CopyFeedUrl]);

        let remote = album_page_actions(library(ReleaseMembershipState::RemoteOnly), FEED_URL);
        assert_eq!(remote.filled.action, AlbumPageAction::DownloadAlbum);
        assert!(remote.filled.available);
        assert!(remote.plain.is_empty());
        assert_eq!(
            actions_of(&remote.menu),
            vec![
                AlbumPageAction::CopyFeedUrl,
                AlbumPageAction::MusicBrainzLookup
            ]
        );

        let in_library = album_page_actions(library(ReleaseMembershipState::InLibrary), FEED_URL);
        assert_eq!(in_library.filled.action, AlbumPageAction::AddToPlaylist);
        assert!(in_library.filled.available);
        assert!(in_library.plain.is_empty());
        assert_eq!(
            actions_of(&in_library.menu),
            vec![
                AlbumPageAction::CopyFeedUrl,
                AlbumPageAction::MusicBrainzLookup,
                AlbumPageAction::RemoveAlbum
            ]
        );
    }

    /// R83-51: a busy download or removal makes the filled action and
    /// "Remove album…" unavailable. The lookup keeps its own availability.
    #[test]
    fn adr_0083_album_page_busy_states_disable_the_filled_action_and_removal() {
        let downloading =
            album_page_actions(library(ReleaseMembershipState::Downloading), FEED_URL);
        assert_eq!(downloading.filled.action, AlbumPageAction::DownloadAlbum);
        assert!(!downloading.filled.available);

        let removing = album_page_actions(library(ReleaseMembershipState::Removing), FEED_URL);
        assert_eq!(removing.filled.action, AlbumPageAction::AddToPlaylist);
        assert!(!removing.filled.available);
        let remove = removing.menu.last().expect("removal stays in the menu");
        assert_eq!(remove.action, AlbumPageAction::RemoveAlbum);
        assert!(!remove.available);

        let lookup_busy = album_page_actions(
            AlbumPageOrigin::Library {
                membership: ReleaseMembershipState::InLibrary,
                musicbrainz_available: false,
            },
            FEED_URL,
        );
        assert!(lookup_busy
            .menu
            .iter()
            .any(|item| item.action == AlbumPageAction::MusicBrainzLookup && !item.available));
    }

    /// R83-51: an album with no feed URL has no "Copy feed URL" item.
    #[test]
    fn adr_0083_album_page_without_feed_url_has_no_copy_item() {
        for feed_url in [None, Some(""), Some("  ")] {
            let actions = album_page_actions(AlbumPageOrigin::Index, feed_url);
            assert!(actions.menu.is_empty(), "{feed_url:?}");
        }
    }

    /// R83-52: "Remove album…" is last and is the only destructive item.
    #[test]
    fn adr_0083_album_page_removal_is_last_and_destructive() {
        let actions = album_page_actions(library(ReleaseMembershipState::InLibrary), FEED_URL);
        let last = actions.menu.last().expect("the menu has items");
        assert_eq!(last.action, AlbumPageAction::RemoveAlbum);
        assert!(last.action.is_destructive());
        assert!(last.action.label().ends_with('…'));
        assert_eq!(
            actions
                .menu
                .iter()
                .filter(|item| item.action.is_destructive())
                .count(),
            1
        );
        assert!(!actions.filled.action.is_destructive());
    }
}
