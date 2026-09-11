//! Settings cache-list observations and refresh admission (ADRs 0040 and 0066).

#![warn(clippy::pedantic)]

use crate::application::errors::command::CommandError;

use super::library::LibraryTree;

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
