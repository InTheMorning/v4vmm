//! Shared playlist detail page contract.
//!
//! This module owns the GPUI-free page shape consumed by the playlist shell.
//! Screens resolve thumbnails and wire playlist commands through shell slots.

#![warn(clippy::pedantic)]

use crate::view_models::library::{
    PlaylistDetailActionsDisplay, PlaylistDetailHeaderDisplay, PlaylistDetailVm, PlaylistTrackRowVm,
};

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
    pub(crate) fn is_empty(&self) -> bool {
        self.detail.is_empty()
    }

    #[must_use]
    pub(crate) fn empty_message(&self) -> &'static str {
        self.detail.empty_message()
    }

    #[must_use]
    pub(crate) fn actions_display(&self) -> PlaylistDetailActionsDisplay {
        self.detail.actions_display()
    }

    #[must_use]
    pub(crate) fn track_rows(&self) -> Vec<PlaylistTrackRowVm<'a>> {
        self.detail.track_rows()
    }
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::view_models::library::PlaylistDetailVm;
    use crate::view_models::playlist_detail::PlaylistDetailPageVm;

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
        let playlist = playlist("Mix");
        let detail = PlaylistDetailVm::new(&playlist, &[]);
        let page = PlaylistDetailPageVm::new(detail, "playlist-detail-scroll");

        assert_eq!(page.scroll_id(), "playlist-detail-scroll");
        assert_eq!(page.playlist_id(), 42);
        assert_eq!(page.header_display().title, "Mix");
        assert_eq!(page.detail_rows(), vec![("Tracks".into(), "0".into())]);
        assert!(page.is_empty());
        assert_eq!(
            page.actions_display().rename_button_id,
            "playlist-rename-42"
        );
    }
}
