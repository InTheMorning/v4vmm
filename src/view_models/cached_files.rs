//! Settings cache-list observations and refresh admission (ADRs 0040 and 0066).

#![warn(clippy::pedantic)]

use crate::application::capability::ExecutionUnavailable;
use crate::application::errors::command::CommandError;

use super::library::{LibraryTrackRowVm, LibraryTree};
use super::startup::StartupAvailability;

#[derive(Clone, Debug)]
pub(crate) enum CachedFileAction {
    Delete(String),
    DeleteAll,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedFileActionDisplay {
    pub(crate) action: CachedFileAction,
    pub(crate) id: String,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: String,
    pub(crate) availability: StartupAvailability,
}

#[derive(Debug)]
pub(crate) enum CachedFileRow {
    Artist(String),
    Track {
        title: String,
        delete: CachedFileActionDisplay,
    },
}

#[derive(Debug, Default, PartialEq, Eq)]
enum LoadState {
    #[default]
    NotRequested,
    Loading,
    Ready,
    Unavailable,
    Failed,
}

/// Retains the last observation while admitting only one read at a time.
#[derive(Debug, Default)]
pub(crate) struct CachedFilesVm {
    tree: LibraryTree,
    count: Option<usize>,
    state: LoadState,
    dirty: bool,
}

impl CachedFilesVm {
    pub(crate) fn rows(
        &self,
        music_dir: &std::path::Path,
        execution: Result<(), ExecutionUnavailable>,
    ) -> Vec<CachedFileRow> {
        let mut rows = Vec::new();
        for artist in &self.tree.artists {
            rows.push(CachedFileRow::Artist(artist.name.clone()));
            for album in &artist.albums {
                for track in &album.tracks {
                    let title = LibraryTrackRowVm::new(track, None).compact_title();
                    let path = track
                        .local_path
                        .as_ref()
                        .map(|path| path.resolve(music_dir));
                    rows.push(CachedFileRow::Track {
                        delete: CachedFileActionDisplay {
                            action: CachedFileAction::Delete(
                                path.as_ref()
                                    .map(|path| path.display().to_string())
                                    .unwrap_or_default(),
                            ),
                            id: format!("del-cached-{}", track.id),
                            label: "Delete",
                            a11y_label: format!("Delete cached file for {title}"),
                            availability: if execution.is_ok() && path.is_some() {
                                StartupAvailability::Available
                            } else {
                                StartupAvailability::Unavailable
                            },
                        },
                        title,
                    });
                }
            }
        }
        rows
    }

    pub(crate) fn delete_all_action(
        &self,
        execution: Result<(), ExecutionUnavailable>,
    ) -> Option<CachedFileActionDisplay> {
        self.has_files().then(|| CachedFileActionDisplay {
            action: CachedFileAction::DeleteAll,
            id: "delete-all-cached-settings".into(),
            label: "Delete All Cached",
            a11y_label: "Delete all cached music files".into(),
            availability: if execution.is_ok() {
                StartupAvailability::Available
            } else {
                StartupAvailability::Unavailable
            },
        })
    }

    /// Ordinary entry admits the first read or a failed-read retry, never a refresh of valid data.
    pub(crate) fn enter(&mut self) {
        if matches!(self.state, LoadState::NotRequested | LoadState::Failed) {
            self.invalidate();
        }
    }

    pub(crate) fn tree(&self) -> &LibraryTree {
        &self.tree
    }

    pub(crate) fn has_files(&self) -> bool {
        self.count.is_some_and(|count| count > 0)
    }

    pub(crate) fn title(&self) -> String {
        self.count.map_or_else(
            || "Cached files".into(),
            |count| format!("Cached files ({count})"),
        )
    }

    pub(crate) fn status(&self) -> Option<String> {
        let message = match self.state {
            LoadState::NotRequested => "App has not read the cached-file list.",
            LoadState::Loading => "App is reading the cached-file list.",
            LoadState::Unavailable => {
                "App could not read the cached-file list because background tools are unavailable. Choose Check again in Background tools above."
            }
            LoadState::Failed => {
                "App could not read the cached-file list from the database. Reopen Settings to try again."
            }
            LoadState::Ready if !self.has_files() => "No cached files.",
            LoadState::Ready => return None,
        };
        let retained = if self.count.is_some() && self.state != LoadState::Ready {
            " App is showing the last loaded list."
        } else {
            ""
        };
        Some(format!("{message}{retained}"))
    }

    pub(crate) fn invalidate(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn begin_load(&mut self) -> bool {
        if !self.dirty || self.state == LoadState::Loading {
            return false;
        }
        self.dirty = false;
        self.state = LoadState::Loading;
        true
    }

    pub(crate) fn complete(&mut self, result: Result<(usize, LibraryTree), CommandError>) {
        if self.state != LoadState::Loading {
            return;
        }
        // A mutation during the read requires one fresh read. Do not replace
        // the previous snapshot with a result that predates that mutation.
        if self.dirty {
            self.state = LoadState::NotRequested;
            return;
        }
        self.state = match result {
            Ok((count, tree)) => {
                self.count = Some(count);
                self.tree = tree;
                LoadState::Ready
            }
            Err(CommandError::Unavailable(_)) => LoadState::Unavailable,
            Err(_) => LoadState::Failed,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0069_cache_entry_reuses_valid_and_in_flight_observations() {
        let mut vm = CachedFilesVm::default();
        vm.enter();
        assert!(vm.begin_load());
        for _ in 0..3 {
            vm.enter();
            assert!(!vm.begin_load());
        }
        vm.complete(Ok((3, LibraryTree::default())));
        assert_eq!(vm.title(), "Cached files (3)");
        for _ in 0..3 {
            vm.enter();
            assert!(!vm.begin_load());
            assert_eq!(vm.title(), "Cached files (3)");
        }
        vm.invalidate(); // An actual mutation still refreshes the mounted observation.
        assert!(vm.begin_load());
        vm.complete(Err(CommandError::Query("failed read".into())));
        assert!(!vm.begin_load());
        vm.enter(); // Existing explicit re-entry retry for a failed read.
        assert!(vm.begin_load());
        vm.complete(Err(CommandError::Unavailable(
            ExecutionUnavailable::RUNTIME,
        )));
        vm.enter();
        assert!(!vm.begin_load()); // Runtime recovery remains the explicit Check again route.
        assert_eq!(vm.title(), "Cached files (3)");
        assert_eq!(
            vm.delete_all_action(Err(ExecutionUnavailable::RUNTIME))
                .unwrap()
                .availability,
            StartupAvailability::Unavailable
        );
    }

    #[test]
    fn adr_0066_unread_and_failed_cache_are_not_reported_as_empty() {
        let mut vm = CachedFilesVm::default();
        assert_eq!(vm.title(), "Cached files");
        assert!(vm.status().unwrap().contains("has not read"));
        vm.invalidate();
        assert!(vm.begin_load());
        assert!(vm.status().unwrap().contains("is reading"));
        vm.complete(Err(CommandError::Query("read failed".into())));
        assert_eq!(vm.title(), "Cached files");
        assert!(vm.status().unwrap().contains("could not read"));
        assert!(!vm.begin_load()); // A failure must not trigger a retry loop.
        vm.invalidate();
        assert!(vm.begin_load());
        vm.complete(Ok((0, LibraryTree::default())));
        assert_eq!(vm.title(), "Cached files (0)");
        assert_eq!(vm.status().as_deref(), Some("No cached files."));
    }

    #[test]
    fn adr_0040_cache_refresh_coalesces_and_discards_invalidated_results() {
        let mut vm = CachedFilesVm::default();
        vm.invalidate();
        assert!(vm.begin_load());
        vm.complete(Ok((2, LibraryTree::default())));
        vm.invalidate();
        assert!(vm.begin_load());
        for _ in 0..3 {
            vm.invalidate();
            assert!(!vm.begin_load());
        }
        vm.complete(Ok((99, LibraryTree::default())));
        assert_eq!(vm.title(), "Cached files (2)");
        assert!(vm.begin_load());
        assert!(!vm.begin_load());
        vm.complete(Ok((1, LibraryTree::default())));
        assert_eq!(vm.title(), "Cached files (1)");
        assert!(!vm.begin_load());
        assert!(vm.status().is_none());
    }

    #[test]
    fn adr_0066_cache_failure_retains_last_observation() {
        let mut vm = CachedFilesVm::default();
        vm.invalidate();
        assert!(vm.begin_load());
        vm.complete(Ok((2, LibraryTree::default())));
        vm.invalidate();
        assert!(vm.begin_load());
        vm.complete(Err(CommandError::Unavailable(
            crate::application::capability::ExecutionUnavailable::RUNTIME,
        )));
        assert!(vm.has_files());
        let status = vm.status().unwrap();
        assert!(status.contains("background tools are unavailable"));
        assert!(status.contains("showing the last loaded list"));
    }
}
