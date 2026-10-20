//! Shared playlist detail page contract.
//!
//! This module owns the GPUI-free page shape consumed by the playlist shell.
//! Screens resolve thumbnails and wire playlist commands through shell slots.

#![warn(clippy::pedantic)]

use crate::db;
use crate::view_models::library::{
    PlaylistDetailActionsDisplay, PlaylistDetailHeaderDisplay, PlaylistDetailVm, PlaylistTrackRowVm,
};

/// The thumbnail URLs of the playlist page rows, in row order (ADR 0084
/// Decision 12, walk fix F5 of 2026-10-20).
///
/// A row reads its track image, else its album image. This is the same
/// order as the lookup of the playlist page row. A URL occurs one time
/// only. A row with no image or with an empty value gives no URL. The
/// caller gives only the rows that the page draws, so the count of image
/// requests stays at the count of drawn rows.
#[must_use]
pub(crate) fn playlist_row_thumbnail_urls<'t>(
    tracks: impl IntoIterator<Item = &'t db::TrackRow>,
) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for track in tracks {
        let url = track
            .track_image_href
            .as_deref()
            .or(track.album_image_href.as_deref())
            .filter(|url| !url.is_empty());
        if let Some(url) = url {
            if !urls.iter().any(|known| known == url) {
                urls.push(url.to_owned());
            }
        }
    }
    urls
}

/// Page-level projection for a playlist detail surface.
pub(crate) struct PlaylistDetailPageVm<'a> {
    detail: PlaylistDetailVm<'a>,
    scroll_id: &'static str,
}

impl<'a> PlaylistDetailPageVm<'a> {
    #[must_use]
    pub(crate) const fn new(detail: PlaylistDetailVm<'a>, scroll_id: &'static str) -> Self {
        Self { detail, scroll_id }
    }

    #[must_use]
    pub(crate) const fn scroll_id(&self) -> &'static str {
        self.scroll_id
    }

    #[must_use]
    pub(crate) fn playlist_id(&self) -> i64 {
        self.detail.playlist_id()
    }

    #[must_use]
    pub(crate) fn header_display(&self) -> PlaylistDetailHeaderDisplay {
        self.detail.header_display()
    }

    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        self.detail.detail_rows()
    }

    #[must_use]
    pub(crate) fn empty_message(&self) -> &'static str {
        self.detail.empty_message()
    }

    #[must_use]
    pub(crate) fn actions_display(&self) -> PlaylistDetailActionsDisplay {
        self.detail.actions_display()
    }

    /// The report of the latest RSS check (ADR 0076 Decision 2).
    #[must_use]
    pub(crate) fn rss_check_report(
        &self,
    ) -> Option<crate::view_models::playlist_rss_check::PlaylistRssCheckReportDisplay> {
        self.detail.rss_check_report()
    }

    #[must_use]
    pub(crate) fn track_rows(&self) -> Vec<PlaylistTrackRowVm<'a>> {
        self.detail.track_rows()
    }
}

/// `is_empty` has no caller outside its own dedicated test: the playlist
/// screen checks slot emptiness instead. Compiled only for tests.
#[cfg(test)]
impl<'a> PlaylistDetailPageVm<'a> {
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.detail.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::view_models::library::PlaylistDetailVm;
    use crate::view_models::playlist_detail::{playlist_row_thumbnail_urls, PlaylistDetailPageVm};

    /// F5 of the walk of 2026-10-20: each playlist row gives its track
    /// image, else its album image, in row order and one time only.
    #[test]
    fn adr_0084_walk_fix_playlist_rows_give_track_image_else_album_image() {
        let row = |track: Option<&str>, album: Option<&str>| db::TrackRow {
            track_image_href: track.map(str::to_owned),
            album_image_href: album.map(str::to_owned),
            ..db::TrackRow::default()
        };
        let rows = [
            row(
                Some("https://img.test/track-1.jpg"),
                Some("https://img.test/album-a.jpg"),
            ),
            row(None, Some("https://img.test/album-a.jpg")),
            row(None, None),
            row(Some(""), None),
            row(None, Some("https://img.test/album-a.jpg")),
            row(Some("https://img.test/track-1.jpg"), None),
            row(None, Some("https://img.test/album-b.jpg")),
        ];

        assert_eq!(
            playlist_row_thumbnail_urls(&rows),
            vec![
                "https://img.test/track-1.jpg".to_owned(),
                "https://img.test/album-a.jpg".to_owned(),
                "https://img.test/album-b.jpg".to_owned(),
            ]
        );
    }

    fn playlist(name: &str) -> db::Playlist {
        db::Playlist {
            id: 42,
            name: name.into(),
            description: None,
            track_count: 0,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn playlist_detail_page_vm_wraps_existing_detail_contract() {
        let mut playlist = playlist("Mix");
        playlist.description = Some("  Local notes  ".into());
        playlist.created_at = 1_712_275_200;
        playlist.updated_at = 1_715_040_000;
        let detail = PlaylistDetailVm::new(&playlist, &[]);
        let page = PlaylistDetailPageVm::new(detail, "playlist-detail-scroll");

        assert_eq!(page.scroll_id(), "playlist-detail-scroll");
        assert_eq!(page.playlist_id(), 42);
        assert_eq!(page.header_display().title, "Mix");
        assert_eq!(
            page.detail_rows(),
            vec![
                ("Tracks".into(), "0".into()),
                ("Created".into(), "Apr 5, 2024".into()),
                ("Modified".into(), "May 7, 2024".into()),
                ("Description".into(), "Local notes".into()),
            ]
        );
        assert!(page.is_empty());
        assert_eq!(
            page.actions_display().rename_button_id,
            "playlist-rename-42"
        );
    }
}
